// components/inventory/InventorySession.js
// One inventory: pick the shelf you are standing at, scan every bottle on it,
// fix what is misplaced, then finish. Finished inventories show the report.

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import useIsMobile from '../hooks/useIsMobile';
import CameraView from '../labels/CameraView';
import BottleHistory from '../labels/BottleHistory';
import useCameraScanner from '../labels/useCameraScanner';
import { canActOnStock } from '../labels/permissions';
import { looksLikeLocationCode } from '../labels/codes';
import { buildInventoryReport } from './inventoryReport';

const RESULT = {
  found: { label: 'On its place', bg: '#c6f6d5', fg: '#22543d', flash: '#48bb78' },
  misplaced: { label: 'Misplaced', bg: '#feebc8', fg: '#7b341e', flash: '#ed8936' },
  unplaced: { label: 'Not placed', bg: '#fefcbf', fg: '#744210', flash: '#ecc94b' },
  inactive: { label: 'Empty / disposed', bg: '#fed7d7', fg: '#822727', flash: '#f56565' },
  unknown: { label: 'Unknown code', bg: '#e2e8f0', fg: '#2d3748', flash: '#a0aec0' },
  missing: { label: 'Missing', bg: '#fed7d7', fg: '#822727', flash: '#f56565' },
};

const fmtWhen = (v) => {
  if (!v) return '';
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleString('ru-RU', {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit',
  });
};

const shortCode = (id) => (id ? id.replace(/-/g, '').slice(0, 8).toUpperCase() : '');

const bottleTitle = (i) => (i.container_id
  ? `${i.reagent_name || '?'} · ${i.batch_number || '?'} #${i.sequence_number ?? '?'}`
  : (i.raw_code || 'Unknown code'));

const Badge = ({ result }) => {
  const r = RESULT[result] || { label: result, bg: '#edf2f7', fg: '#2d3748' };
  return (
    <span style={{ background: r.bg, color: r.fg, fontWeight: 700, fontSize: '0.72rem', padding: '2px 8px', borderRadius: '999px', whiteSpace: 'nowrap' }}>
      {r.label}
    </span>
  );
};

const card = { background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '12px 14px' };
const h3 = { fontSize: '0.95rem', fontWeight: 800, color: '#1a365d', margin: '0 0 8px' };

/** Scanned bottle line with the matching correction button */
const ItemRow = ({ item, onResolve, busy, open }) => {
  const [showHistory, setShowHistory] = useState(false);
  const fixable = open && ((item.result === 'misplaced') || (item.result === 'unplaced'));
  const unplaceable = !open && item.result === 'missing' && !item.resolution;
  return (
    <li style={{ padding: '8px 0', borderBottom: '1px solid #f1f5f9' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', alignItems: 'flex-start' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 600, color: '#1a202c', fontSize: '0.9rem', overflowWrap: 'anywhere' }}>{bottleTitle(item)}</div>
          <div style={{ fontSize: '0.78rem', color: '#718096', overflowWrap: 'anywhere' }}>
            {item.result === 'misplaced' && <>Here: {item.location} · recorded at {item.expected_location || '?'}</>}
            {item.result === 'unplaced' && <>Here: {item.location} · not placed in Chelate</>}
            {item.result === 'inactive' && <>Here: {item.location} · recorded as empty or disposed</>}
            {item.result === 'missing' && <>Recorded at {item.expected_location || '?'}</>}
            {item.result === 'found' && <>{item.location}</>}
            {item.result === 'unknown' && <>Scanned at {item.location}</>}
            {item.resolution === 'moved_here' && <> · moved here ✓</>}
            {item.resolution === 'placed_here' && <> · placed here ✓</>}
            {item.resolution === 'unplaced' && <> · marked as not placed ✓</>}
          </div>
        </div>
        <Badge result={item.result} />
      </div>
      <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginTop: '6px' }}>
        {fixable && (
          <Button size="small" variant="primary" disabled={busy}
            onClick={() => onResolve(item, item.result === 'misplaced' ? 'move_here' : 'place_here')}>
            {item.result === 'misplaced' ? 'Move here' : 'Place here'}
          </Button>
        )}
        {unplaceable && (
          <Button size="small" variant="secondary" disabled={busy} onClick={() => onResolve(item, 'unplace')}
            title="The bottle is not where Chelate says: remove its shelf record">
            Mark as not placed
          </Button>
        )}
        {item.container_id && (
          <button type="button" onClick={() => setShowHistory(v => !v)} aria-expanded={showHistory}
            style={{ background: 'none', border: 'none', color: '#2b6cb0', fontSize: '0.8rem', fontWeight: 600, cursor: 'pointer', padding: '4px 0' }}>
            {showHistory ? 'Hide history' : 'History'}
          </button>
        )}
      </div>
      {showHistory && item.container_id && (
        <div style={{ marginTop: '6px', paddingLeft: '4px' }}><BottleHistory containerId={item.container_id} limit={20} /></div>
      )}
    </li>
  );
};

const InventorySession = ({ id, user, onBack }) => {
  const isMobile = useIsMobile();
  const canAct = canActOnStock(user);
  const fileRef = useRef(null);
  const busyRef = useRef(false);
  const [detail, setDetail] = useState(null);
  const [error, setError] = useState('');
  const [shelfId, setShelfId] = useState('');
  const [last, setLast] = useState(null);
  const [busy, setBusy] = useState(false);
  const [manual, setManual] = useState('');
  const [confirming, setConfirming] = useState(false);
  const [notes, setNotes] = useState('');
  const [showFound, setShowFound] = useState(false);
  const [shelfNote, setShelfNote] = useState('');

  const load = useCallback(async () => {
    try {
      const d = await api.getInventorySession(id);
      setDetail(d);
      setShelfId(prev => (prev && d.positions.some(p => p.id === prev)) ? prev : (d.positions[0]?.id || ''));
    } catch (e) {
      setError(e.message || 'Could not load the inventory check');
    }
  }, [id]);

  useEffect(() => { load(); }, [load]);

  const open = detail?.session?.status === 'open';

  // A shelf's QR label picks the shelf being checked (when it is part of this check)
  const switchShelf = useCallback(async (code, positions) => {
    const r = await api.resolveCode(code);
    const loc = r.location;
    if (!loc) throw new Error('Not a location label');
    if (loc.kind !== 'position') {
      throw new Error(`That is a ${loc.kind === 'zone' ? 'cabinet' : 'room'} label. Scan the label of the shelf you are at.`);
    }
    if (!positions.some(p => p.id === loc.id)) {
      throw new Error(`${loc.path} is not part of this check.`);
    }
    setShelfId(loc.id);
    setLast(null);
    setShelfNote(loc.path);
  }, []);

  const submitCode = useCallback(async (code) => {
    if (busyRef.current || !shelfId) return;
    busyRef.current = true;
    setBusy(true);
    setError('');
    setShelfNote('');
    try {
      if (looksLikeLocationCode(code)) {
        await switchShelf(code, detail ? detail.positions : []);
        if (navigator.vibrate) navigator.vibrate(60);
        return;
      }
      const r = await api.inventoryScan(id, code, shelfId);
      setLast(r);
      if (navigator.vibrate && r.result !== 'found') navigator.vibrate([60, 80, 60]);
      await load();
    } catch (e) {
      setLast(null);
      setError(e.message || 'Scan failed');
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, [id, shelfId, load, detail, switchShelf]);

  const scanner = useCameraScanner({ onCode: submitCode, continuous: true, cooldownMs: 3000 });
  const { liveSupported, start, stop, cameraOn } = scanner;

  useEffect(() => {
    if (open && liveSupported && isMobile && canAct) start();
    if (!open) stop();
  }, [open, liveSupported, isMobile, canAct, start, stop]);

  const onPhoto = async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = '';
    if (!file) return;
    setError('');
    try {
      const texts = await scanner.decodeFile(file);
      if (texts.length > 0) await submitCode(texts[0]);
      else setError('No label found in the photo. Hold the phone 10–15 cm from the label, without glare.');
    } catch (err) {
      setError(err.message || 'Could not read the photo');
    }
  };

  const resolve = async (item, action) => {
    setBusy(true);
    setError('');
    try {
      await api.resolveInventoryItem(id, item.id, action);
      if (last && last.item && last.item.id === item.id) setLast(null);
      await load();
    } catch (e) {
      setError(e.message || 'Could not apply the correction');
    } finally {
      setBusy(false);
    }
  };

  const finish = async () => {
    setBusy(true);
    setError('');
    try {
      stop();
      const d = await api.closeInventory(id, notes);
      setDetail(d);
      setConfirming(false);
      setLast(null);
    } catch (e) {
      setError(e.message || 'Could not finish the check');
    } finally {
      setBusy(false);
    }
  };

  const cancel = async () => {
    setBusy(true);
    try {
      stop();
      await api.cancelInventory(id);
      onBack();
    } catch (e) {
      setError(e.message || 'Could not cancel');
      setBusy(false);
    }
  };

  const printReport = () => {
    const w = window.open('', '_blank');
    if (!w) { setError('The browser blocked the report window. Allow pop-ups for this site.'); return; }
    w.document.open();
    w.document.write(buildInventoryReport(detail));
    w.document.close();
  };

  const items = useMemo(() => detail?.items || [], [detail]);
  const toFind = useMemo(
    () => (detail?.expected || []).filter(e => !e.scanned && e.position_id === shelfId),
    [detail, shelfId]
  );
  const onShelf = useMemo(() => (detail?.expected || []).filter(e => e.position_id === shelfId), [detail, shelfId]);

  if (!detail) {
    return <div style={{ padding: '20px', color: error ? '#c53030' : '#718096' }}>{error || 'Loading inventory check…'}</div>;
  }
  const s = detail.session;
  const c = detail.counts;
  const lastStyle = last ? (RESULT[last.result] || RESULT.unknown) : null;

  const header = (
    <div style={{ marginBottom: '12px' }}>
      <button type="button" onClick={onBack}
        style={{ background: 'none', border: 'none', color: '#4f46e5', fontWeight: 600, cursor: 'pointer', padding: '4px 0', marginBottom: '4px' }}>
        ← All inventory checks
      </button>
      <div style={{ fontSize: '1.2rem', fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere' }}>{s.scope_name}</div>
      <div style={{ fontSize: '0.8rem', color: '#718096' }}>
        Started {fmtWhen(s.started_at)}{s.started_by_name ? ` by ${s.started_by_name}` : ''}
        {s.closed_at && <> · {s.status === 'cancelled' ? 'cancelled' : 'finished'} {fmtWhen(s.closed_at)}{s.closed_by_name ? ` by ${s.closed_by_name}` : ''}</>}
      </div>
    </div>
  );

  const chip = (label, value, color) => (
    <div style={{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '10px', padding: '6px 10px', minWidth: '72px' }}>
      <div style={{ fontSize: '1.1rem', fontWeight: 800, color }}>{value}</div>
      <div style={{ fontSize: '0.7rem', color: '#718096', whiteSpace: 'nowrap' }}>{label}</div>
    </div>
  );

  // ==================== FINISHED: REPORT ====================
  if (!open) {
    const sectionsOrder = ['missing', 'misplaced', 'unplaced', 'inactive', 'unknown'];
    return (
      <div>
        {header}
        {s.status === 'cancelled' && (
          <div style={{ ...card, background: '#f7fafc', color: '#4a5568', marginBottom: '12px' }}>This check was cancelled; it is not a valid record.</div>
        )}
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginBottom: '12px' }}>
          {chip('Expected', c.expected, '#1a365d')}
          {chip('Found', c.found, '#2f855a')}
          {chip('Missing', c.missing, c.missing ? '#c53030' : '#1a365d')}
          {chip('Misplaced', c.misplaced, c.misplaced ? '#c05621' : '#1a365d')}
          {chip('Not placed', c.unplaced, '#1a365d')}
          {chip('Unknown', c.unknown, '#1a365d')}
        </div>
        {s.notes && <div style={{ ...card, marginBottom: '12px', fontSize: '0.85rem', whiteSpace: 'pre-wrap' }}>{s.notes}</div>}
        {error && <div role="alert" style={{ color: '#c53030', marginBottom: '8px' }}>{error}</div>}
        <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
          <Button variant="primary" onClick={printReport}>Print report</Button>
        </div>
        {sectionsOrder.map(r => {
          const rows = items.filter(i => i.result === r);
          if (rows.length === 0) return null;
          return (
            <div key={r} style={{ ...card, marginBottom: '12px' }}>
              <h3 style={h3}>{RESULT[r].label} ({rows.length})</h3>
              {r === 'missing' && canAct && s.status === 'closed' && (
                <div style={{ fontSize: '0.78rem', color: '#718096', marginBottom: '4px' }}>
                  Recorded in this place but not scanned. If a bottle is really gone, mark it as not placed;
                  its history keeps the record.
                </div>
              )}
              <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
                {rows.map(i => <ItemRow key={i.id} item={i} busy={busy || !canAct || s.status !== 'closed'} onResolve={resolve} open={false} />)}
              </ul>
            </div>
          );
        })}
        {items.some(i => i.result === 'found') && (
          <div style={{ ...card, marginBottom: '12px' }}>
            <button type="button" onClick={() => setShowFound(v => !v)} aria-expanded={showFound}
              style={{ ...h3, background: 'none', border: 'none', padding: 0, cursor: 'pointer', margin: 0 }}>
              {RESULT.found.label} ({items.filter(i => i.result === 'found').length}) {showFound ? '▴' : '▾'}
            </button>
            {showFound && (
              <ul style={{ listStyle: 'none', margin: '8px 0 0', padding: 0 }}>
                {items.filter(i => i.result === 'found').map(i => <ItemRow key={i.id} item={i} busy onResolve={resolve} open={false} />)}
              </ul>
            )}
          </div>
        )}
      </div>
    );
  }

  // ==================== OPEN: SCANNING ====================
  const shelf = detail.positions.find(p => p.id === shelfId);
  const scannedOnShelf = onShelf.filter(e => e.scanned).length;

  return (
    <div>
      {header}

      {!canAct && (
        <div style={{ ...card, background: '#fffaf0', marginBottom: '12px', fontSize: '0.85rem' }}>
          You can view this check; scanning needs permission to edit batches.
        </div>
      )}

      {detail.positions.length === 0 ? (
        <div style={{ ...card, marginBottom: '12px' }}>This place has no shelves (positions) yet. Add positions in Storage first.</div>
      ) : (
        <div style={{ marginBottom: '10px' }}>
          <label htmlFor="inv-shelf" style={{ display: 'block', fontSize: '0.75rem', fontWeight: 700, color: '#4a5568', marginBottom: '4px' }}>
            Shelf you are checking
          </label>
          <select id="inv-shelf" value={shelfId} onChange={e => { setShelfId(e.target.value); setLast(null); setShelfNote(''); }}
            disabled={detail.positions.length === 1}
            style={{ width: '100%', height: '44px', padding: '0 10px', border: '1px solid #e2e8f0', borderRadius: '10px', background: 'white' }}>
            {detail.positions.map(p => (
              <option key={p.id} value={p.id}>
                {s.scope_type === 'room' ? `${p.zone_name} / ${p.name}` : s.scope_type === 'zone' ? p.name : `${p.zone_name} / ${p.name}`}
              </option>
            ))}
          </select>
          {shelf && (
            <div style={{ fontSize: '0.78rem', color: '#718096', marginTop: '4px' }}>
              {scannedOnShelf} of {onShelf.length} recorded bottle{onShelf.length !== 1 ? 's' : ''} on this shelf scanned
              {detail.positions.length > 1 && ' · scan a shelf QR label to switch shelf'}
            </div>
          )}
          {shelfNote && (
            <div role="status" style={{ marginTop: '6px', background: '#ebf8ff', border: '1px solid #bee3f8', color: '#2a4365', borderRadius: '8px', padding: '6px 10px', fontSize: '0.82rem', fontWeight: 600 }}>
              Shelf: {shelfNote}
            </div>
          )}
        </div>
      )}

      {canAct && detail.positions.length > 0 && (
        <>
          <canvas ref={scanner.canvasRef} style={{ display: 'none' }} />
          <input ref={fileRef} type="file" accept="image/*" capture="environment" onChange={onPhoto} style={{ display: 'none' }} />
          {liveSupported && <CameraView scanner={scanner} hidden={!isMobile && !cameraOn} flash={lastStyle ? lastStyle.flash : null} />}
          {!liveSupported && (
            <div style={{ background: '#ebf8ff', border: '1px solid #bee3f8', color: '#2a4365', borderRadius: '12px', padding: '10px 12px', fontSize: '0.82rem' }}>
              Live scanning needs HTTPS. Take a photo of each label instead.
            </div>
          )}

          {last && (
            <div role="status" style={{ ...card, marginTop: '10px', borderColor: lastStyle.flash, borderWidth: '2px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                <div style={{ fontWeight: 700, color: '#1a202c', overflowWrap: 'anywhere' }}>
                  {last.container ? `${last.container.reagent_name} · ${last.container.batch_number} #${last.container.sequence_number}` : bottleTitle(last.item)}
                </div>
                <Badge result={last.result} />
              </div>
              <div style={{ fontSize: '0.82rem', color: '#4a5568', marginTop: '4px' }}>
                {last.result === 'found' && (last.already_scanned ? 'Already counted.' : 'Counted.')}
                {last.result === 'misplaced' && `Chelate has it at ${last.item.expected_location || 'another shelf'}.`}
                {last.result === 'unplaced' && 'Chelate has no shelf for this bottle.'}
                {last.result === 'inactive' && `Chelate records it as ${last.container ? last.container.container_status : 'empty'}.`}
                {last.result === 'unknown' && 'Not a Chelate label, or the bottle was deleted.'}
              </div>
              {(last.result === 'misplaced' || last.result === 'unplaced') && !last.item.resolution && (
                <Button size="small" variant="primary" disabled={busy} style={{ marginTop: '8px' }}
                  onClick={() => resolve(last.item, last.result === 'misplaced' ? 'move_here' : 'place_here')}>
                  {last.result === 'misplaced' ? 'Move here' : 'Place here'}
                </Button>
              )}
            </div>
          )}

          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '10px' }}>
            {liveSupported && !isMobile && !cameraOn && (
              <Button variant="secondary" onClick={start} style={{ height: '44px' }}>Use webcam</Button>
            )}
            <Button variant={liveSupported ? 'secondary' : 'primary'} onClick={() => fileRef.current && fileRef.current.click()} style={{ height: '44px' }} disabled={busy}>
              {liveSupported ? (isMobile ? 'Take a photo instead' : 'Read a photo of the label') : 'Take a photo of the label'}
            </Button>
            <form onSubmit={e => { e.preventDefault(); if (manual.trim()) { submitCode(manual.trim()); setManual(''); } }} style={{ display: 'flex', gap: '8px' }}>
              <input value={manual} onChange={e => setManual(e.target.value)} placeholder="Code under the symbol"
                aria-label="Label code" autoFocus={!isMobile} autoComplete="off" spellCheck={false}
                style={{ flex: 1, minWidth: 0, height: '44px', padding: '0 12px', border: '1px solid #e2e8f0', borderRadius: '10px', fontFamily: 'monospace' }} />
              <Button type="submit" variant="secondary" disabled={!manual.trim() || busy} style={{ height: '44px' }}>Add</Button>
            </form>
          </div>
        </>
      )}

      {error && (
        <div role="alert" style={{ marginTop: '10px', background: '#fff5f5', border: '1px solid #fed7d7', color: '#c53030', borderRadius: '10px', padding: '10px 12px', fontSize: '0.875rem' }}>
          {error}
        </div>
      )}

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', margin: '12px 0' }}>
        {chip('Found', c.found, '#2f855a')}
        {chip('To find', c.remaining, c.remaining ? '#c05621' : '#2f855a')}
        {chip('Misplaced', c.misplaced, c.misplaced ? '#c05621' : '#1a365d')}
        {chip('Not placed', c.unplaced, '#1a365d')}
        {chip('Unknown', c.unknown, '#1a365d')}
      </div>

      {shelf && (
        <div style={{ ...card, marginBottom: '12px' }}>
          <h3 style={h3}>Still to find on {shelf.name} ({toFind.length})</h3>
          {toFind.length === 0 ? (
            <div style={{ fontSize: '0.85rem', color: '#2f855a' }}>Every bottle recorded on this shelf is scanned.</div>
          ) : (
            <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
              {toFind.map(e => (
                <li key={e.container_id} style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', padding: '6px 0', borderBottom: '1px solid #f1f5f9', fontSize: '0.85rem' }}>
                  <span style={{ minWidth: 0, overflowWrap: 'anywhere' }}>{e.reagent_name} · {e.batch_number} #{e.sequence_number}</span>
                  <span style={{ fontFamily: 'monospace', color: '#718096', whiteSpace: 'nowrap' }}>{e.short_code || shortCode(e.container_id)}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      {items.length > 0 && (
        <div style={{ ...card, marginBottom: '12px' }}>
          <h3 style={h3}>Scanned ({items.length})</h3>
          <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
            {items.map(i => <ItemRow key={i.id} item={i} busy={busy || !canAct} onResolve={resolve} open />)}
          </ul>
        </div>
      )}

      {canAct && (
        <div style={{ ...card, marginBottom: '24px' }}>
          {!confirming ? (
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', justifyContent: 'space-between' }}>
              <Button variant="primary" onClick={() => setConfirming(true)} disabled={busy}>Finish check</Button>
              <Button variant="ghost" onClick={cancel} disabled={busy} title="Discard: started by mistake">Cancel check</Button>
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ fontSize: '0.875rem', color: '#2d3748' }}>
                {c.remaining > 0
                  ? <><b>{c.remaining}</b> bottle{c.remaining !== 1 ? 's' : ''} recorded in {s.scope_name} {c.remaining !== 1 ? 'were' : 'was'} not scanned and will be listed as <b>missing</b>.</>
                  : 'Every recorded bottle was scanned.'}
              </div>
              <textarea value={notes} onChange={e => setNotes(e.target.value)} rows={2} maxLength={1000}
                placeholder="Notes for the record (optional)"
                style={{ width: '100%', boxSizing: 'border-box', padding: '8px 10px', border: '1px solid #e2e8f0', borderRadius: '10px', fontFamily: 'inherit' }} />
              <div style={{ display: 'flex', gap: '8px' }}>
                <Button variant="primary" onClick={finish} disabled={busy} loading={busy}>Finish</Button>
                <Button variant="secondary" onClick={() => setConfirming(false)} disabled={busy}>Keep scanning</Button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default InventorySession;
