// components/labels/ReceiveSession.js
// Receiving goods by scanning the manufacturer's own code on each bottle.
//
// Bottles of one batch carry the same code, so identical scans add up:
// "Merck 105426, batch K55640426 × 26". Each group is then given a reagent
// (remembered per article, so only a new article needs a person), the quantity
// in one bottle and an expiry date, and "Receive" creates the batch with its
// bottles in one step. Labels are printed afterwards for the new bottles.
// Scanning a shelf's QR label first puts the new bottles on that shelf.
// A batch number that looks like one already in Chelate (l/I, a cut-off digit,
// the pack typed after it, or the same number under another reagent) has to be
// chosen or confirmed as new before it is received.

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import useIsMobile from '../hooks/useIsMobile';
import CameraView from './CameraView';
import LabelPrintModal from './LabelPrintModal';
import useCameraScanner from './useCameraScanner';
import ArticleBackfill from './ArticleBackfill';
import { canonUnit, casOk, unitFactor } from './codes';

const UNITS = ['g', 'mg', 'kg', 'mL', 'L', 'pcs'];
const EMPTY_CHECK = { exact: null, similar: [], elsewhere: [] };
const SIMILAR_WHY = {
  case: 'Only upper and lower case differ.',
  lookalike: 'I, l and 1, or O and 0, look alike.',
  cut: 'A character or two at the end differ.',
  pack: 'The pack size seems to be typed after the number.',
  same: '',
};
const SERIES_NAME = { merck: 'Merck', sigma: 'Sigma-Aldrich', other: '' };

const input = {
  width: '100%', height: '40px', padding: '0 10px', border: '1px solid #e2e8f0',
  borderRadius: '8px', boxSizing: 'border-box', fontSize: '0.9rem', background: 'white',
};
const label = { display: 'block', fontSize: '0.72rem', fontWeight: 700, color: '#4a5568', margin: '10px 0 3px' };

const num = (v) => {
  const n = parseFloat(String(v).replace(',', '.'));
  return Number.isFinite(n) ? n : NaN;
};

const dateOnly = (iso) => (iso ? String(iso).slice(0, 10) : '');

const fmtQty = (v) => String(Math.round(v * 1e6) / 1e6);

/** What the batch check was (or must be) made for: reagent and batch number */
const CHECKING = 'Checking the batch number…';

const checkKeyOf = (g) => `${g.reagent ? g.reagent.id : (g.draft ? 'new' : '')}|${g.batchNumber.trim()}`;

/** An existing batch fills the quantity and unit the person has not entered yet */
const fillFromBatch = (g, batch) => {
  if (!batch) return {};
  const patch = {};
  if (!g.packSize && batch.pack_size) patch.packSize = String(batch.pack_size);
  if (!g.unit) patch.unit = canonUnit(batch.unit) || batch.unit;
  return patch;
};

const articleTitle = (code) => {
  const series = SERIES_NAME[code.series] || '';
  const cat = code.catalog || (code.gtin ? `GTIN ${code.gtin}` : '?');
  const pack = code.pack ? ` · ${code.pack.raw}` : '';
  return `${series ? `${series} ` : ''}${cat}${pack}${code.grade_suffix ? `-${code.grade_suffix}` : ''}`;
};

/** Article and pack code of a scanned bottle, for the server's quantity suggestion */
const articleOf = (code) => (code.catalog && code.pack ? { series: code.series, catalog: code.catalog, pack: code.pack.raw } : null);

/** Merck 1.00983 (as printed), other numbering as is */
const articleName = (a) => (a.series === 'merck' && /^\d{6}$/.test(a.catalog) ? `Merck ${a.catalog[0]}.${a.catalog.slice(1)}` : a.catalog);

const MAX_BOTTLES = 500;
const countOf = (g) => Number(g.count) || 0;

/** Bottles with the same article, batch and pack are one group */
const sigOf = (code) => [code.series, code.catalog || code.gtin || '', code.batch || '', code.pack ? code.pack.raw : ''].join('|');

let groupSeq = 0;

const baseUnitOf = (unit) => (unit === 'mL' || unit === 'L' ? 'mL' : 'g');

/** First values of a new group: what the code and the stored article already say */
const seedGroup = (scan) => {
  const code = scan.code;
  const existing = scan.batch;
  const pack = existing && existing.pack_size
    ? { value: existing.pack_size, unit: canonUnit(existing.unit) || existing.unit }
    : scan.suggested_pack;
  const sig = sigOf(code);
  groupSeq += 1;
  return {
    sig,
    key: `${sig}#${groupSeq}`,
    scan,
    count: 1,
    reagent: scan.reagent ? { id: scan.reagent.id, name: scan.reagent.name, cas: scan.reagent.cas_number } : null,
    draft: null,
    batchNumber: code.batch || '',
    packSize: pack ? String(pack.value) : '',
    unit: pack ? pack.unit : (existing ? canonUnit(existing.unit) || existing.unit : ''),
    expiry: code.expiry || dateOnly(existing && existing.expiry_date),
    check: { exact: existing || null, similar: scan.similar_batches || [], elsewhere: scan.batch_elsewhere || [] },
    checkKey: `${scan.reagent ? scan.reagent.id : ''}|${(code.batch || '').trim()}`,
    confirmedNew: '', // checkKeyOf(g) the person confirmed as a new batch
    expanded: false, // a complete card shows one line until the person opens it
    state: 'ready', // ready | saving | done | error
    error: '',
    result: null,
  };
};

const groupProblem = (g) => {
  if (!g.reagent && !g.draft) return 'Choose the reagent';
  if (g.draft) {
    if (!g.draft.name.trim()) return 'Name the new reagent';
    if (g.draft.cas.trim() && !casOk(g.draft.cas)) return 'CAS number is not valid (check the digits)';
  }
  if (!g.batchNumber.trim()) return 'Enter the batch number';
  const key = checkKeyOf(g);
  if (g.checkKey !== key) return CHECKING;
  const c = g.check;
  if (!c.exact && (c.similar.length > 0 || c.elsewhere.length > 0) && g.confirmedNew !== key) {
    return 'A similar batch is already in Chelate: choose it or confirm a new batch';
  }
  if (!(num(g.packSize) > 0)) return 'Enter the quantity in one bottle';
  if (!g.unit) return 'Choose the unit';
  if (c.exact && unitFactor(g.unit, c.exact.unit) == null) {
    return `This batch is kept in ${canonUnit(c.exact.unit) || c.exact.unit}: choose a unit that converts to it`;
  }
  if (!(countOf(g) >= 1)) return 'Enter the number of bottles';
  if (countOf(g) > MAX_BOTTLES) return `At most ${MAX_BOTTLES} bottles at once`;
  return '';
};

// ---------- reagent picker ----------

const ReagentPicker = ({ group, onChange }) => {
  const [q, setQ] = useState('');
  const [found, setFound] = useState([]);
  const [searching, setSearching] = useState(false);
  const [articles, setArticles] = useState({});
  const [creating, setCreating] = useState(!!group.draft);
  const code = group.scan.code;

  useEffect(() => {
    const term = q.trim();
    if (term.length < 2) { setFound([]); return undefined; }
    let cancelled = false;
    const t = setTimeout(async () => {
      setSearching(true);
      try {
        const r = await api.searchReagents(term);
        if (!cancelled) setFound(r);
        // which products each reagent already holds: another article of the same maker is usually another grade
        const a = await api.articlesForReagents(r.map(x => x.id)).catch(() => ({}));
        if (!cancelled) setArticles(a);
      } catch (_) {
        if (!cancelled) setFound([]);
      } finally {
        if (!cancelled) setSearching(false);
      }
    }, 250);
    return () => { cancelled = true; clearTimeout(t); };
  }, [q]);

  if (group.reagent) {
    const others = !group.scan.reagent && code.catalog
      ? (group.reagent.articles || []).filter(a => a.series === code.series && a.catalog !== code.catalog)
      : [];
    return (
      <>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '8px', background: '#f0fff4', border: '1px solid #c6f6d5', borderRadius: '8px', padding: '8px 10px' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 700, color: '#22543d', overflowWrap: 'anywhere' }}>{group.reagent.name}</div>
          {group.reagent.cas && <div style={{ fontSize: '0.75rem', color: '#4a5568' }}>CAS {group.reagent.cas}</div>}
        </div>
        {group.state !== 'done' && (
          <button type="button" onClick={() => onChange({ reagent: null })}
            style={{ background: 'none', border: 'none', color: '#2b6cb0', fontWeight: 700, cursor: 'pointer', whiteSpace: 'nowrap' }}>
            Change
          </button>
        )}
      </div>
      {others.length > 0 && (
        <div role="alert" style={{ fontSize: '0.78rem', color: '#7b341e', background: '#fffaf0', border: '1px solid #fbd38d', borderRadius: '8px', padding: '6px 10px', marginTop: '6px' }}>
          {group.reagent.name} already holds {others.map(articleName).join(', ')}. Another article of the same maker
          is usually another grade or concentration: check the label, or create a new reagent.
        </div>
      )}
      </>
    );
  }

  if (creating) {
    const d = group.draft || { name: '', cas: '', formula: '' };
    const set = (patch) => onChange({ draft: { ...d, ...patch } });
    const casBad = d.cas.trim() && !casOk(d.cas);
    return (
      <div style={{ border: '1px solid #bee3f8', background: '#ebf8ff', borderRadius: '8px', padding: '8px 10px' }}>
        <div style={{ fontSize: '0.75rem', fontWeight: 800, color: '#2a4365' }}>NEW REAGENT</div>
        <label style={label}>Name
          <input style={input} value={d.name} onChange={e => set({ name: e.target.value })} placeholder="e.g. trans-Cinnamaldehyde" />
        </label>
        <label style={label}>CAS number (printed on the bottle)
          <input style={{ ...input, borderColor: casBad ? '#fc8181' : '#e2e8f0' }} value={d.cas} onChange={e => set({ cas: e.target.value })} placeholder="14371-10-9" inputMode="numeric" />
        </label>
        {casBad && <div style={{ fontSize: '0.75rem', color: '#c53030', marginTop: '3px' }}>The check digit does not match. Recheck the number.</div>}
        <label style={label}>Formula (optional)
          <input style={input} value={d.formula} onChange={e => set({ formula: e.target.value })} placeholder="C9H8O" />
        </label>
        <button type="button" onClick={() => { setCreating(false); onChange({ draft: null }); }}
          style={{ background: 'none', border: 'none', color: '#2b6cb0', fontWeight: 700, padding: '8px 0 0', cursor: 'pointer' }}>
          Pick an existing reagent instead
        </button>
      </div>
    );
  }

  return (
    <div>
      <input style={input} value={q} onChange={e => setQ(e.target.value)} placeholder="Search reagent by name, CAS or formula" aria-label="Search reagent" />
      {searching && <div style={{ fontSize: '0.75rem', color: '#718096', marginTop: '4px' }}>Searching…</div>}
      {found.map(r => (
        <button key={r.id} type="button"
          onClick={() => onChange({ reagent: { id: r.id, name: r.name, cas: r.cas_number, articles: articles[r.id] || [] }, draft: null })}
          style={{ display: 'block', width: '100%', textAlign: 'left', padding: '8px 10px', marginTop: '4px', border: '1px solid #e2e8f0', borderRadius: '8px', background: 'white', cursor: 'pointer' }}>
          <div style={{ fontWeight: 700, color: '#1a365d', overflowWrap: 'anywhere' }}>{r.name}</div>
          <div style={{ fontSize: '0.75rem', color: '#718096' }}>{[r.cas_number && `CAS ${r.cas_number}`, r.formula].filter(Boolean).join(' · ')}</div>
          {(articles[r.id] || []).length > 0 && (
            <div style={{ fontSize: '0.75rem', color: '#4a5568' }}>Articles: {articles[r.id].map(articleName).join(', ')}</div>
          )}
        </button>
      ))}
      {q.trim().length >= 2 && !searching && found.length === 0 && (
        <div style={{ fontSize: '0.8rem', color: '#718096', marginTop: '4px' }}>No reagent found.</div>
      )}
      <button type="button" onClick={() => { setCreating(true); onChange({ draft: { name: q.trim(), cas: '', formula: '' } }); }}
        style={{ background: 'none', border: 'none', color: '#2b6cb0', fontWeight: 700, padding: '8px 0 0', cursor: 'pointer' }}>
        + New reagent
      </button>
    </div>
  );
};

// ---------- one group ----------

const GroupCard = ({ g, onChange, onRemove }) => {
  const code = g.scan.code;
  const problem = groupProblem(g);
  const done = g.state === 'done';
  const existing = g.check.exact;
  const merckPack = code.series === 'merck' && code.pack && !code.pack.unit && !existing && !(num(g.packSize) > 0);
  const key = checkKeyOf(g);
  const c = g.check;
  const similarShown = !done && !existing && (c.similar.length > 0 || c.elsewhere.length > 0);
  const factor = existing && g.unit ? unitFactor(g.unit, existing.unit) : null;
  // A complete card is one line; it opens when something is missing or the person asks
  const showForm = g.expanded || g.state === 'error' || (!!problem && problem !== CHECKING);
  const converted = existing && factor != null && canonUnit(g.unit) !== canonUnit(existing.unit) && num(g.packSize) > 0
    ? `${fmtQty(num(g.packSize) * factor)} ${canonUnit(existing.unit) || existing.unit}` : '';

  // Once something was missing the card stays open until the person closes it,
  // so it does not fold away under their fingers when the last field is filled
  useEffect(() => {
    if (!done && problem && problem !== CHECKING && !g.expanded) onChange({ expanded: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [problem, done]);

  // Re-check the batch number whenever it or the reagent changes
  useEffect(() => {
    if (done || key === g.checkKey) return undefined;
    const batch = g.batchNumber.trim();
    if (!batch) { onChange({ check: EMPTY_CHECK, checkKey: key }); return undefined; }
    let cancelled = false;
    const t = setTimeout(async () => {
      let check = EMPTY_CHECK;
      try {
        check = await api.checkBatch(g.reagent ? g.reagent.id : null, batch, articleOf(code));
      } catch (_) {
        // the server checks again when receiving
      }
      if (cancelled) return;
      const patch = { check, checkKey: key, ...fillFromBatch(g, check.exact) };
      // a pack received before, or a round Merck code read with the chosen reagent's unit
      if (!check.exact && !g.packSize && check.suggested_pack) {
        patch.packSize = String(check.suggested_pack.value);
        if (!g.unit) patch.unit = check.suggested_pack.unit;
      }
      onChange(patch);
    }, 300);
    return () => { cancelled = true; clearTimeout(t); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, done]);

  return (
    <li style={{ background: 'white', border: `1px solid ${done ? '#c6f6d5' : g.state === 'error' ? '#fed7d7' : '#e2e8f0'}`, borderRadius: '12px', padding: '12px', listStyle: 'none' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere' }}>
            {g.reagent ? g.reagent.name : (g.draft && g.draft.name) || articleTitle(code)}
          </div>
          <div style={{ fontSize: '0.78rem', color: '#718096', overflowWrap: 'anywhere' }}>
            {g.reagent || (g.draft && g.draft.name) ? articleTitle(code) : ''}
            {g.batchNumber ? `${g.reagent || (g.draft && g.draft.name) ? ' · ' : ''}batch ${g.batchNumber}` : ''}
          </div>
        </div>
        {!done && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexShrink: 0 }}>
            <button type="button" aria-label="One bottle less" onClick={() => onChange({ count: Math.max(1, countOf(g) - 1) })}
              style={{ width: '32px', height: '32px', borderRadius: '8px', border: '1px solid #e2e8f0', background: 'white', fontSize: '1.1rem', cursor: 'pointer' }}>−</button>
            <input aria-label="Bottles" inputMode="numeric" value={g.count}
              onChange={e => { const d = e.target.value.replace(/\D/g, '').slice(0, 3); onChange({ count: d === '' ? '' : Number(d) }); }}
              onFocus={e => e.target.select()}
              style={{ width: '48px', height: '34px', textAlign: 'center', fontWeight: 800, fontSize: '1.1rem', border: '1px solid #e2e8f0', borderRadius: '8px', padding: 0 }} />
            <button type="button" aria-label="One bottle more" onClick={() => onChange({ count: Math.min(MAX_BOTTLES, countOf(g) + 1) })}
              style={{ width: '32px', height: '32px', borderRadius: '8px', border: '1px solid #e2e8f0', background: 'white', fontSize: '1.1rem', cursor: 'pointer' }}>+</button>
          </div>
        )}
      </div>

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginTop: '8px' }}>
        {existing && <Chip tone="blue">Batch already in Chelate: {existing.bottles} bottle{existing.bottles !== 1 ? 's' : ''}. These are added.</Chip>}
        {!existing && g.reagent && !similarShown && <Chip tone="green">New batch</Chip>}
        {!g.reagent && !g.draft && <Chip tone="orange">New article</Chip>}
        {g.scan.expired && <Chip tone="red">Expired {code.expiry}</Chip>}
        {code.warnings.map(w => <Chip key={w} tone="orange">{w}</Chip>)}
      </div>

      {similarShown && (
        <div role="alert" style={{ marginTop: '8px', background: '#fffaf0', border: '1px solid #fbd38d', borderRadius: '8px', padding: '8px 10px', fontSize: '0.82rem', color: '#7b341e' }}>
          {c.similar.map(s => (
            <div key={s.batch_id} style={{ marginBottom: '6px' }}>
              Batch <b>{s.batch_number}</b> of this reagent is already in Chelate ({s.bottles} bottle{s.bottles !== 1 ? 's' : ''}). {SIMILAR_WHY[s.reason]}
              <div>
                <button type="button" onClick={() => onChange({ batchNumber: s.batch_number, confirmedNew: '' })} style={linkBtn}>
                  Add to {s.batch_number}
                </button>
              </div>
            </div>
          ))}
          {c.elsewhere.map(s => (
            <div key={s.batch_id} style={{ marginBottom: '6px' }}>
              Batch <b>{s.batch_number}</b> is recorded under <b>{s.reagent_name}</b>. {SIMILAR_WHY[s.reason]}
              {g.reagent ? (
                <div>Check the reagent: a different grade or concentration is a different reagent.</div>
              ) : (
                <div>
                  <button type="button" style={linkBtn}
                    onClick={() => onChange({ reagent: { id: s.reagent_id, name: s.reagent_name }, draft: null, batchNumber: s.batch_number, confirmedNew: '' })}>
                    Use {s.reagent_name}
                  </button>
                </div>
              )}
            </div>
          ))}
          {g.confirmedNew === key ? (
            <div style={{ fontWeight: 700, color: '#276749' }}>✓ {g.batchNumber.trim()} will be received as a new batch</div>
          ) : (
            <button type="button" onClick={() => onChange({ confirmedNew: key })} style={linkBtn}>
              No, {g.batchNumber.trim()} is a new batch
            </button>
          )}
        </div>
      )}

      {done && g.result && (
        <div style={{ marginTop: '8px', color: '#276749', fontSize: '0.85rem', fontWeight: 700 }}>
          ✓ {countOf(g)} bottle{countOf(g) !== 1 ? 's' : ''} received
          {g.result.placed > 0 ? `, ${g.result.placed} placed on the shelf` : ''}
          {g.result.warnings.map(w => <div key={w} style={{ color: '#975a16', fontWeight: 400 }}>{w}</div>)}
        </div>
      )}
      {g.state === 'error' && <div role="alert" style={{ marginTop: '8px', color: '#c53030', fontSize: '0.85rem' }}>{g.error}</div>}

      {!done && !showForm && (
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '8px', gap: '8px' }}>
          <div style={{ fontSize: '0.85rem', color: '#276749', fontWeight: 700, minWidth: 0, overflowWrap: 'anywhere' }}>
            {countOf(g)} × {g.packSize} {g.unit}{g.expiry ? ` · expires ${g.expiry}` : ''}
          </div>
          <button type="button" onClick={() => onChange({ expanded: true })} style={{ ...linkBtn, padding: 0, whiteSpace: 'nowrap' }}>
            Edit
          </button>
        </div>
      )}

      {!done && showForm && (
        <>
          <div style={{ marginTop: '10px' }}>
            <ReagentPicker group={g} onChange={onChange} />
          </div>

          <div style={{ display: 'flex', gap: '8px' }}>
            <label style={{ ...label, flex: 1 }}>Quantity in one bottle
              <input style={input} value={g.packSize} onChange={e => onChange({ packSize: e.target.value })} inputMode="decimal" placeholder="2.5" />
            </label>
            <label style={{ ...label, width: '96px' }}>Unit
              <select style={input} value={g.unit} onChange={e => onChange({ unit: e.target.value })}>
                <option value="">–</option>
                {UNITS.map(u => <option key={u} value={u}>{u}</option>)}
                {g.unit && !UNITS.includes(g.unit) && <option value={g.unit}>{g.unit}</option>}
              </select>
            </label>
          </div>
          {converted && (
            <div style={{ fontSize: '0.75rem', color: '#2a4365', marginTop: '3px' }}>
              The batch is kept in {canonUnit(existing.unit) || existing.unit}: each bottle is recorded as {converted}.
            </div>
          )}
          {merckPack && (
            <div style={{ fontSize: '0.75rem', color: '#975a16', marginTop: '3px' }}>
              Pack code {code.pack.raw} is not a quantity: take it from the label.
            </div>
          )}

          <div style={{ display: 'flex', gap: '8px' }}>
            <label style={{ ...label, flex: 1 }}>Batch number
              <input style={input} value={g.batchNumber} onChange={e => onChange({ batchNumber: e.target.value })} />
            </label>
            <label style={{ ...label, flex: 1 }}>Expiry date
              <input style={input} type="date" value={g.expiry || ''} onChange={e => onChange({ expiry: e.target.value })} />
            </label>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '10px', gap: '8px' }}>
            <div style={{ fontSize: '0.8rem', color: problem ? '#975a16' : '#276749', minWidth: 0 }}>
              {problem || `${countOf(g)} × ${g.packSize} ${g.unit}`}
            </div>
            <div style={{ display: 'flex', gap: '14px', flexShrink: 0 }}>
              {!problem && (
                <button type="button" onClick={() => onChange({ expanded: false })} style={{ ...linkBtn, padding: 0 }}>Done</button>
              )}
              <button type="button" onClick={onRemove}
                style={{ background: 'none', border: 'none', color: '#c53030', fontWeight: 700, cursor: 'pointer', whiteSpace: 'nowrap', padding: 0 }}>
                Remove
              </button>
            </div>
          </div>
        </>
      )}
    </li>
  );
};

const linkBtn = { background: 'none', border: 'none', color: '#2b6cb0', fontWeight: 700, padding: '4px 0 0', cursor: 'pointer', textAlign: 'left' };

const TONES = {
  green: ['#f0fff4', '#276749'], blue: ['#ebf8ff', '#2a4365'], orange: ['#fffaf0', '#7b341e'], red: ['#fff5f5', '#c53030'],
};
const Chip = ({ tone, children }) => (
  <span style={{ background: TONES[tone][0], color: TONES[tone][1], borderRadius: '999px', padding: '3px 10px', fontSize: '0.75rem', fontWeight: 700 }}>
    {children}
  </span>
);

// ---------- session ----------

const ReceiveSession = ({ initialScan, onDone }) => {
  const isMobile = useIsMobile();
  const fileRef = useRef(null);
  const busyRef = useRef(false);
  const [groups, setGroups] = useState(() => (initialScan ? [seedGroup(initialScan)] : []));
  const [notes, setNotes] = useState([]);
  const [manual, setManual] = useState('');
  const [busy, setBusy] = useState(false);
  const [shelf, setShelf] = useState(null);
  const [saving, setSaving] = useState(false);
  const [printing, setPrinting] = useState(false);

  const note = useCallback((tone, text) => {
    setNotes(prev => [{ id: Date.now() + Math.random(), tone, text }, ...prev].slice(0, 4));
  }, []);

  const update = useCallback((key, patch) => {
    // Editing a group that failed clears its error; an explicit state in the patch wins
    setGroups(prev => prev.map(g => (g.key === key ? { ...g, ...patch, state: patch.state || (g.state === 'error' ? 'ready' : g.state) } : g)));
  }, []);

  const handle = useCallback(async (raw) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    try {
      const r = await api.resolveCode(raw);
      if (r.location) {
        if (r.location.kind === 'position') {
          setShelf(r.location);
          note('ok', `New bottles will go on ${r.location.path}`);
        } else {
          note('warn', 'That is a cabinet or room label. Scan the label of a shelf.');
        }
        return;
      }
      if (r.matches.length > 0) {
        note('warn', 'This bottle already has a Chelate label: it is in the system.');
        return;
      }
      if (!r.supplier) {
        note('warn', 'Not a manufacturer code.');
        return;
      }
      const code = r.supplier.code;
      if (code.format === 'gtin') {
        note('warn', r.supplier.reagent
          ? `Barcode of ${r.supplier.reagent.name}: it has no batch. Scan the square Data Matrix instead.`
          : 'This barcode has no batch number. Scan the square Data Matrix instead.');
        return;
      }
      const sig = sigOf(code);
      setGroups(prev => {
        const hit = prev.find(g => g.sig === sig && g.state !== 'done');
        if (hit) return prev.map(g => (g === hit ? { ...g, count: Math.min(MAX_BOTTLES, countOf(g) + 1) } : g));
        // newest first: it is the one the person is looking at
        return [seedGroup(r.supplier), ...prev];
      });
      if (navigator.vibrate) navigator.vibrate(40);
    } catch (e) {
      note('error', (e.message || 'Could not read the code').replace(/^(Bad Request|Not Found): /, ''));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, [note]);

  const scanner = useCameraScanner({ onCode: handle, continuous: true, cooldownMs: 1500 });
  const { liveSupported, start, stop, cameraOn } = scanner;

  useEffect(() => {
    if (liveSupported && isMobile) start();
    return () => stop();
  }, [liveSupported, isMobile, start, stop]);

  const onPhoto = async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = '';
    if (!file) return;
    try {
      const texts = await scanner.decodeFile(file);
      if (texts.length > 0) await handle(texts[0]);
      else note('error', 'No code found in the photo. Hold the phone 10–15 cm from the symbol, without glare.');
    } catch (err) {
      note('error', err.message || 'Could not read the photo');
    }
  };

  const pending = useMemo(() => groups.filter(g => g.state !== 'done'), [groups]);
  const receivedGroups = useMemo(() => groups.filter(g => g.state === 'done'), [groups]);
  const readyAll = pending.length > 0 && pending.every(g => !groupProblem(g));
  const totalBottles = pending.reduce((n, g) => n + countOf(g), 0);
  const firstProblem = pending.map(g => [g, groupProblem(g)]).find(([, p]) => p);
  const compact = groups.length > 0;
  const barFixed = isMobile && pending.length > 0;
  const newIds = useMemo(() => receivedGroups.flatMap(g => (g.result ? g.result.container_ids : [])), [receivedGroups]);

  const receiveAll = async () => {
    setSaving(true);
    stop();
    for (const g of groups) {
      if (g.state === 'done' || groupProblem(g)) continue;
      update(g.key, { state: 'saving', error: '' });
      try {
        let reagent = g.reagent;
        if (!reagent) {
          const created = await api.createReagent({
            name: g.draft.name.trim(),
            cas_number: g.draft.cas.trim() || undefined,
            formula: g.draft.formula.trim() || undefined,
          });
          const id = (created.data && created.data.id) || created.id;
          if (!id) throw new Error('The reagent was not created');
          reagent = { id, name: g.draft.name.trim(), cas: g.draft.cas.trim() };
          update(g.key, { reagent, draft: null });
        }
        const code = g.scan.code;
        const body = {
          reagent_id: reagent.id,
          batch_number: g.batchNumber.trim(),
          bottles: countOf(g),
          pack_size: num(g.packSize),
          unit: g.unit,
          expiry_date: g.expiry || undefined,
          cat_number: code.catalog || undefined,
          supplier: SERIES_NAME[code.series] || undefined,
          pcode: code.pcode || undefined,
          position_id: shelf ? shelf.id : undefined,
          source: 'scan',
          confirm_new_batch: g.confirmedNew === checkKeyOf(g) || undefined,
        };
        if (code.catalog) {
          body.remember = {
            series: code.series,
            catalog: code.catalog,
            gtin: code.gtin || undefined,
            pack_unit: code.series === 'merck' ? baseUnitOf(g.unit) : undefined,
            // the quantity of this Merck pack code is offered next time
            pack_code: code.series === 'merck' && code.pack && /^\d{1,10}$/.test(code.pack.raw) ? code.pack.raw : undefined,
          };
        }
        const result = await api.receiveGoods(body);
        update(g.key, { state: 'done', result, expanded: false });
      } catch (e) {
        update(g.key, { state: 'error', error: e.message || 'Could not receive this batch' });
      }
    }
    setSaving(false);
  };

  return (
    <div>
      <div style={{ background: '#f0fff4', border: '2px solid #38a169', borderRadius: '12px', padding: compact ? '6px 12px' : '10px 14px', marginBottom: '10px' }}>
        <div style={{ fontSize: '0.7rem', fontWeight: 800, letterSpacing: '0.06em', color: '#2f855a' }}>RECEIVING GOODS</div>
        <div style={{ fontSize: compact ? '0.8rem' : '0.85rem', color: '#4a5568', marginTop: '2px' }}>
          {compact ? null : 'Scan the square Data Matrix on each bottle. Bottles of one batch add up. '}
          {shelf ? <>New bottles go on <b>{shelf.path}</b>.</> : 'Scan a shelf label to put the new bottles on that shelf.'}
        </div>
      </div>

      {groups.length === 0 && <ArticleBackfill />}

      <canvas ref={scanner.canvasRef} style={{ display: 'none' }} />
      <input ref={fileRef} type="file" accept="image/*" capture="environment" onChange={onPhoto} style={{ display: 'none' }} />
      {liveSupported && <CameraView scanner={scanner} hidden={!isMobile && !cameraOn} compact={compact} />}
      {!liveSupported && (
        <div style={{ background: '#ebf8ff', border: '1px solid #bee3f8', color: '#2a4365', borderRadius: '12px', padding: '10px 12px', fontSize: '0.82rem' }}>
          Live scanning needs HTTPS. Take a photo of each code instead, or use a USB scanner in the field below.
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '10px' }}>
        {liveSupported && !isMobile && !cameraOn && (
          <Button variant="secondary" onClick={start} style={{ height: '44px' }}>Use webcam</Button>
        )}
        {compact && liveSupported ? (
          <button type="button" onClick={() => fileRef.current && fileRef.current.click()} disabled={busy}
            style={{ ...linkBtn, padding: 0, alignSelf: 'center' }}>
            {isMobile ? 'Take a photo instead' : 'Read a photo of the code'}
          </button>
        ) : (
          <Button variant="secondary" onClick={() => fileRef.current && fileRef.current.click()} style={{ height: '44px' }} disabled={busy}>
            {isMobile ? 'Take a photo instead' : 'Read a photo of the code'}
          </Button>
        )}
        <form onSubmit={e => { e.preventDefault(); if (manual.trim()) { handle(manual.trim()); setManual(''); } }} style={{ display: 'flex', gap: '8px' }}>
          <input value={manual} onChange={e => setManual(e.target.value)} placeholder="Scan with a USB scanner or paste a code"
            aria-label="Manufacturer code" autoFocus={!isMobile} autoComplete="off" spellCheck={false}
            style={{ ...input, flex: 1, minWidth: 0, height: '44px', fontFamily: 'monospace' }} />
          <Button type="submit" variant="secondary" disabled={!manual.trim() || busy} style={{ height: '44px' }}>Add</Button>
        </form>
      </div>

      {notes.length > 0 && (
        <ul aria-label="Messages" style={{ listStyle: 'none', margin: '10px 0 0', padding: 0, display: 'flex', flexDirection: 'column', gap: '6px' }}>
          {notes.map((n, i) => {
            const t = { ok: ['#f0fff4', '#276749'], warn: ['#fffaf0', '#7b341e'], error: ['#fff5f5', '#c53030'] }[n.tone];
            return (
              <li key={n.id} role={i === 0 ? 'status' : undefined}
                style={{ background: t[0], color: t[1], borderRadius: '10px', padding: '8px 10px', fontSize: '0.85rem' }}>
                {n.text}
              </li>
            );
          })}
        </ul>
      )}

      {groups.length > 0 && (
        <ul aria-label="Batches to receive" style={{ margin: '12px 0 0', padding: 0, display: 'flex', flexDirection: 'column', gap: '10px' }}>
          {groups.map(g => (
            <GroupCard key={g.key} g={g}
              onChange={patch => update(g.key, patch)}
              onRemove={() => setGroups(prev => prev.filter(x => x.key !== g.key))} />
          ))}
        </ul>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '14px' }}>
        {pending.length > 0 && (
          // On a phone the button stays above the tab bar, so a scanned bottle is received without scrolling
          <div style={barFixed ? {
            position: 'fixed', left: 0, right: 0, zIndex: 950, bottom: 'calc(64px + env(safe-area-inset-bottom))',
            padding: '8px 16px', background: 'rgba(255, 255, 255, 0.97)', borderTop: '1px solid #e2e8f0',
            boxShadow: '0 -2px 10px rgba(26, 54, 93, 0.08)',
          } : undefined}>
            {firstProblem && !saving && (
              <div style={{ fontSize: '0.78rem', color: '#975a16', marginBottom: '6px', overflowWrap: 'anywhere' }}>
                {(firstProblem[0].reagent && firstProblem[0].reagent.name) || articleTitle(firstProblem[0].scan.code)}: {firstProblem[1]}
              </div>
            )}
            <Button variant="primary" onClick={receiveAll} disabled={!readyAll || saving} loading={saving} style={{ width: '100%', height: '48px' }}>
              Receive {totalBottles} bottle{totalBottles !== 1 ? 's' : ''}
              {pending.length > 1 ? ` in ${pending.length} batches` : ''}
            </Button>
          </div>
        )}
        {newIds.length > 0 && (
          <Button variant="secondary" onClick={() => setPrinting(true)} style={{ width: '100%', height: '46px' }}>
            Print labels for {newIds.length} new bottle{newIds.length !== 1 ? 's' : ''}
          </Button>
        )}
        <Button variant={pending.length === 0 && newIds.length > 0 ? 'primary' : 'secondary'}
          onClick={() => { stop(); onDone(newIds.length); }} style={{ width: '100%', height: '46px' }}>
          {pending.length > 0 ? 'Close without receiving' : 'Done'}
        </Button>
        {barFixed && <div aria-hidden="true" style={{ height: firstProblem ? '96px' : '72px' }} />}
      </div>

      {printing && (
        <LabelPrintModal
          isOpen={printing}
          onClose={() => setPrinting(false)}
          title="Labels for the new bottles"
          subtitle={`${newIds.length} bottle${newIds.length !== 1 ? 's' : ''} just received`}
          selector={{ container_ids: newIds }}
        />
      )}
    </div>
  );
};

export default ReceiveSession;
