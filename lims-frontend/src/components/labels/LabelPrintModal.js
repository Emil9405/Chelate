// components/labels/LabelPrintModal.js
// Prints Data Matrix bottle labels on A4 (plain paper to cut, or sticker sheets).
//
// `selector` picks the bottles: { batch_ids } | { zone_id } | { container_ids }.
// `batch` (optional) lets the modal create containers for a batch that has none:
// labels belong to bottles, so a batch must be split into containers first.

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Modal from '../Modal';
import Button from '../Button';
import { api } from '../../services/api';
import { LAYOUTS, perSheet, buildCells, buildSheetDocument } from './labelSheet';
import { DataMatrixIcon } from './LabelIcons';
import { PrinterIcon } from '../Icons';

const SETTINGS_KEY = 'chelate.labels.settings';

const loadSettings = () => {
  try {
    const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) || '{}');
    return {
      layout: LAYOUTS[s.layout] ? s.layout : 'medium',
      cutGuides: s.cutGuides !== false,
    };
  } catch (_) {
    return { layout: 'medium', cutGuides: true };
  }
};

const saveSettings = (s) => {
  try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(s)); } catch (_) { /* private mode */ }
};

const STATUS_LABEL = { full: 'sealed', partial: 'opened', empty: 'empty' };

const locationText = (l) => (l.position_name
  ? [l.room_name, l.zone_name, l.position_name].filter(Boolean).join(' / ')
  : 'not placed');

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);

// The global stylesheet gives every input width: 100% and padding
const checkbox = { width: '16px', height: '16px', padding: 0, margin: 0, flexShrink: 0, cursor: 'pointer' };

const PREVIEW_WIDTH_PX = 840; // 210 mm sheet + screen margins in the preview document

const LabelPrintModal = ({ isOpen, onClose, title, subtitle, selector, batch, onContainersChanged }) => {
  const [labels, setLabels] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState(new Set());
  const [settings, setSettings] = useState(loadSettings);
  const [copies, setCopies] = useState(1);
  const [skip, setSkip] = useState(0);
  const [includeEmpty, setIncludeEmpty] = useState(false);
  const [previewHtml, setPreviewHtml] = useState('');
  const [printing, setPrinting] = useState(false);
  const [splitting, setSplitting] = useState(false);
  const previewBox = useRef(null);
  const [previewScale, setPreviewScale] = useState(0.6);

  const layout = LAYOUTS[settings.layout];
  const selectorKey = JSON.stringify(selector || {});

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api.getContainerLabels({ ...JSON.parse(selectorKey), include_empty: includeEmpty });
      setLabels(data);
      setSelected(new Set(data.map(l => l.container_id)));
    } catch (e) {
      setError(e.message || 'Failed to load labels');
      setLabels([]);
    } finally {
      setLoading(false);
    }
  }, [selectorKey, includeEmpty]);

  useEffect(() => { if (isOpen) load(); }, [isOpen, load]);

  const chosen = useMemo(() => labels.filter(l => selected.has(l.container_id)), [labels, selected]);
  const cells = useMemo(
    () => buildCells(chosen, { copies, skip }),
    [chosen, copies, skip]
  );
  const per = perSheet(layout);
  const sheetCount = Math.max(1, Math.ceil(cells.length / per));

  // Preview: first sheet only (symbols are generated in wasm, keep it light)
  useEffect(() => {
    if (!isOpen || chosen.length === 0) { setPreviewHtml(''); return undefined; }
    let cancelled = false;
    const t = setTimeout(async () => {
      try {
        const html = await buildSheetDocument(cells.slice(0, per), layout, {
          cutGuides: settings.cutGuides, assetBase: window.location.origin,
        });
        if (!cancelled) setPreviewHtml(html);
      } catch (e) {
        if (!cancelled) setError(e.message || 'Could not draw the labels');
      }
    }, 200);
    return () => { cancelled = true; clearTimeout(t); };
  }, [isOpen, cells, per, layout, settings.cutGuides, chosen.length]);

  // Fit the A4 preview into its column
  useEffect(() => {
    if (!isOpen || !previewBox.current) return undefined;
    const el = previewBox.current;
    const update = () => setPreviewScale(Math.min(1, el.clientWidth / PREVIEW_WIDTH_PX));
    update();
    const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(update) : null;
    if (ro) ro.observe(el);
    return () => ro && ro.disconnect();
  }, [isOpen, previewHtml]);

  const updateSettings = (patch) => {
    setSettings(prev => {
      const next = { ...prev, ...patch };
      saveSettings(next);
      return next;
    });
    if (patch.layout) setSkip(0);
  };

  const toggle = (id) => setSelected(prev => {
    const next = new Set(prev);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });

  const handlePrint = async () => {
    if (chosen.length === 0) return;
    // Open the window inside the click: pop-up blockers allow it only here
    const w = window.open('', '_blank');
    if (!w) {
      setError('The browser blocked the print window. Allow pop-ups for this site and try again.');
      return;
    }
    w.document.write('<p style="font-family:sans-serif;padding:24px">Preparing labels…</p>');
    setPrinting(true);
    try {
      const html = await buildSheetDocument(cells, layout, {
        cutGuides: settings.cutGuides, autoPrint: true,
        assetBase: window.location.origin, title: title || 'Labels',
      });
      w.document.open();
      w.document.write(html);
      w.document.close();
    } catch (e) {
      w.close();
      setError(e.message || 'Could not prepare labels');
    } finally {
      setPrinting(false);
    }
  };

  const split = async (packSize) => {
    if (!batch) return;
    setSplitting(true);
    setError('');
    try {
      await api.splitBatchIntoContainers(batch.id, packSize);
      if (onContainersChanged) onContainersChanged(batch.id);
      await load();
    } catch (e) {
      setError(e.message || 'Could not create containers');
    } finally {
      setSplitting(false);
    }
  };

  if (!isOpen) return null;

  // Group the checklist by batch
  const groups = [];
  for (const l of labels) {
    const last = groups[groups.length - 1];
    if (last && last.batch_id === l.batch_id) last.items.push(l);
    else groups.push({ batch_id: l.batch_id, title: `${l.reagent_name} · ${l.batch_number}`, items: [l] });
  }

  const noContainersYet = !loading && labels.length === 0 && batch
    && !(batch.container_count > 0) && (batch.quantity || 0) > 0;
  const packCount = batch?.pack_size > 0 ? Math.ceil(batch.quantity / batch.pack_size) : 0;

  const label = { display: 'block', fontSize: '0.75rem', fontWeight: 600, color: '#4a5568', marginBottom: '4px' };
  const input = {
    width: '100%', height: '38px', padding: '0 10px', border: '1px solid #e2e8f0',
    borderRadius: '8px', background: 'white', color: '#1a202c', boxSizing: 'border-box',
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="xlarge"
      title={(
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '10px' }}>
          <DataMatrixIcon size={20} color="#3182ce" />
          <span>
            Bottle labels
            {subtitle && <span style={{ display: 'block', fontSize: '0.8rem', fontWeight: 500, color: '#718096' }}>{subtitle}</span>}
          </span>
        </span>
      )}
      footer={(
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '12px', flexWrap: 'wrap', width: '100%' }}>
          <span style={{ fontSize: '0.85rem', color: '#4a5568' }}>
            {cells.length - skip} label{cells.length - skip !== 1 ? 's' : ''} · {sheetCount} A4 sheet{sheetCount !== 1 ? 's' : ''}
          </span>
          <div style={{ display: 'flex', gap: '8px' }}>
            <Button variant="secondary" onClick={onClose}>Close</Button>
            <Button
              variant="primary"
              onClick={handlePrint}
              disabled={chosen.length === 0 || printing}
              loading={printing}
              icon={<PrinterIcon size={16} />}
            >
              Print
            </Button>
          </div>
        </div>
      )}
    >
      <div style={{ display: 'grid', gridTemplateColumns: '340px 1fr', gap: '20px', alignItems: 'start' }}>
        {/* Settings + bottle list */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', minWidth: 0 }}>
          <div>
            <label style={label} htmlFor="label-layout">Label size</label>
            <select
              id="label-layout"
              value={settings.layout}
              onChange={e => updateSettings({ layout: e.target.value })}
              style={input}
            >
              {Object.values(LAYOUTS).map(l => (
                <option key={l.id} value={l.id}>{l.name} ({perSheet(l)}/A4)</option>
              ))}
            </select>
            <div style={{ fontSize: '0.75rem', color: '#718096', marginTop: '4px' }}>{layout.hint}</div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: '10px' }}>
            <div>
              <label style={label} htmlFor="label-copies">Copies per bottle</label>
              <input id="label-copies" type="number" min="1" max="10" step="1" value={copies}
                onChange={e => setCopies(Math.min(10, Math.max(1, parseInt(e.target.value, 10) || 1)))}
                style={input} />
            </div>
            <div>
              <label style={label} htmlFor="label-skip" title="Leave already used cells of a sticker sheet empty">
                Skip first cells
              </label>
              <input id="label-skip" type="number" min="0" max={per - 1} step="1" value={skip}
                onChange={e => setSkip(Math.min(per - 1, Math.max(0, parseInt(e.target.value, 10) || 0)))}
                style={input} />
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '0.85rem', color: '#2d3748' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input type="checkbox" style={checkbox} checked={settings.cutGuides}
                onChange={e => updateSettings({ cutGuides: e.target.checked })} />
              Cut guides (dashed borders, for plain paper)
            </label>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input type="checkbox" style={checkbox} checked={includeEmpty} onChange={e => setIncludeEmpty(e.target.checked)} />
              Include empty bottles
            </label>
          </div>

          <div style={{ fontSize: '0.75rem', color: '#718096', background: '#f7fafc', borderRadius: '8px', padding: '8px 10px', lineHeight: 1.45 }}>
            In the print dialog: scale <b>100%</b> (Actual size), margins <b>None</b>.
            Other settings shift the labels relative to the cut guides and sticker cells.
          </div>

          {/* Bottles */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
              <span style={{ ...label, marginBottom: 0 }}>Bottles ({selected.size}/{labels.length})</span>
              <span style={{ display: 'flex', gap: '6px' }}>
                <button type="button" onClick={() => setSelected(new Set(labels.map(l => l.container_id)))}
                  style={{ border: '1px solid #3182ce', color: '#3182ce', background: 'white', borderRadius: '6px', fontSize: '0.75rem', padding: '3px 8px', cursor: 'pointer' }}>
                  All
                </button>
                <button type="button" onClick={() => setSelected(new Set())}
                  style={{ border: '1px solid #e2e8f0', color: '#718096', background: 'white', borderRadius: '6px', fontSize: '0.75rem', padding: '3px 8px', cursor: 'pointer' }}>
                  None
                </button>
              </span>
            </div>
            <div style={{ maxHeight: '260px', overflowY: 'auto', border: '1px solid #e2e8f0', borderRadius: '8px' }}>
              {loading && <div style={{ padding: '12px', color: '#a0aec0', fontSize: '0.85rem' }}>Loading…</div>}
              {!loading && labels.length === 0 && !noContainersYet && (
                <div style={{ padding: '12px', color: '#718096', fontSize: '0.85rem' }}>
                  No bottles to label{includeEmpty ? '' : ' (empty ones are hidden)'}.
                </div>
              )}
              {noContainersYet && (
                <div style={{ padding: '12px', fontSize: '0.85rem', color: '#2d3748', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  <div>
                    This batch has no containers yet. A label belongs to one bottle,
                    so record the bottles first:
                  </div>
                  <Button size="small" variant="secondary" disabled={splitting}
                    onClick={() => split(batch.quantity)}>
                    One bottle — {fmtQty(batch.quantity)} {batch.unit}
                  </Button>
                  {packCount > 1 && (
                    <Button size="small" variant="secondary" disabled={splitting}
                      onClick={() => split(batch.pack_size)}>
                      {packCount} bottles × {fmtQty(batch.pack_size)} {batch.unit}
                    </Button>
                  )}
                </div>
              )}
              {groups.map(g => (
                <div key={g.batch_id}>
                  {groups.length > 1 && (
                    <div style={{ padding: '6px 10px', background: '#f7fafc', fontSize: '0.75rem', fontWeight: 700, color: '#2d3748', borderBottom: '1px solid #edf2f7' }}>
                      {g.title}
                    </div>
                  )}
                  {g.items.map(l => (
                    <label key={l.container_id} style={{
                      display: 'flex', alignItems: 'center', gap: '8px', padding: '6px 10px',
                      borderBottom: '1px solid #f7fafc', fontSize: '0.8rem', cursor: 'pointer',
                    }}>
                      <input type="checkbox" style={checkbox} checked={selected.has(l.container_id)} onChange={() => toggle(l.container_id)} />
                      <span style={{ fontWeight: 600, minWidth: '34px' }}>#{l.sequence_number}</span>
                      <span style={{ whiteSpace: 'nowrap' }}>{fmtQty(l.quantity)} {l.unit}</span>
                      <span style={{ color: '#718096' }}>{STATUS_LABEL[l.container_status] || l.container_status}</span>
                      <span style={{ color: '#a0aec0', marginLeft: 'auto', minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                        title={locationText(l)}>
                        {locationText(l)}
                      </span>
                    </label>
                  ))}
                </div>
              ))}
            </div>
          </div>

          {error && (
            <div role="alert" style={{ color: '#c53030', background: '#fff5f5', border: '1px solid #fed7d7', borderRadius: '8px', padding: '8px 10px', fontSize: '0.85rem' }}>
              {error}
            </div>
          )}
        </div>

        {/* Preview of the first sheet */}
        <div ref={previewBox} style={{ minWidth: 0 }}>
          <div style={{ ...label, display: 'flex', justifyContent: 'space-between' }}>
            <span>Preview{sheetCount > 1 ? ' — sheet 1' : ''}</span>
          </div>
          <div style={{
            width: '100%', height: `${Math.round(1180 * previewScale)}px`, overflow: 'hidden',
            borderRadius: '8px', background: '#e2e8f0',
          }}>
            {previewHtml ? (
              <iframe
                title="Label sheet preview"
                srcDoc={previewHtml}
                sandbox=""
                style={{
                  width: `${PREVIEW_WIDTH_PX}px`, height: '1180px', border: 0,
                  transform: `scale(${previewScale})`, transformOrigin: 'top left', pointerEvents: 'none',
                }}
              />
            ) : (
              <div style={{ padding: '24px', color: '#718096', fontSize: '0.85rem' }}>
                {chosen.length === 0 ? 'Select bottles to see the sheet.' : 'Drawing labels…'}
              </div>
            )}
          </div>
        </div>
      </div>
    </Modal>
  );
};

export default LabelPrintModal;
