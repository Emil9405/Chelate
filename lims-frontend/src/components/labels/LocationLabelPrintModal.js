// components/labels/LocationLabelPrintModal.js
// Prints QR labels for storage locations (room, cabinet, shelf) on A4.
//
// `selector` picks the places: { room_id | zone_id | position_id, include_children }.
// A scan of these labels shows the place and what is on it (Scan page), moves
// bottles onto a shelf, and picks the shelf in an inventory check.

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Modal from '../Modal';
import Button from '../Button';
import useIsMobile from '../hooks/useIsMobile';
import { api } from '../../services/api';
import { LOCATION_LAYOUTS, perSheet, buildCells, buildSheetDocument } from './labelSheet';
import { PrinterIcon } from '../Icons';

const SETTINGS_KEY = 'chelate.locationLabels.settings';

const loadSettings = () => {
  try {
    const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) || '{}');
    return { layout: LOCATION_LAYOUTS[s.layout] ? s.layout : 'shelf', cutGuides: s.cutGuides !== false };
  } catch (_) {
    return { layout: 'shelf', cutGuides: true };
  }
};

const saveSettings = (s) => {
  try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(s)); } catch (_) { /* private mode */ }
};

const KIND = { room: 'Room', zone: 'Cabinet', position: 'Shelf' };
const KIND_COLOR = { room: '#6b46c1', zone: '#2b6cb0', position: '#2f855a' };

// The global stylesheet gives every input width: 100% and padding
const checkbox = { width: '16px', height: '16px', padding: 0, margin: 0, flexShrink: 0, cursor: 'pointer' };

const PREVIEW_WIDTH_PX = 840;

const LocationLabelPrintModal = ({ isOpen, onClose, title, subtitle, selector }) => {
  const isMobile = useIsMobile();
  const [labels, setLabels] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState(new Set());
  const [settings, setSettings] = useState(loadSettings);
  const [copies, setCopies] = useState(1);
  const [skip, setSkip] = useState(0);
  const [previewHtml, setPreviewHtml] = useState('');
  const [printing, setPrinting] = useState(false);
  const previewBox = useRef(null);
  const [previewScale, setPreviewScale] = useState(0.6);

  const layout = LOCATION_LAYOUTS[settings.layout];
  const selectorKey = JSON.stringify(selector || {});

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api.getLocationLabels(JSON.parse(selectorKey));
      setLabels(data);
      setSelected(new Set(data.map(l => l.payload)));
    } catch (e) {
      setError(e.message || 'Failed to load labels');
      setLabels([]);
    } finally {
      setLoading(false);
    }
  }, [selectorKey]);

  useEffect(() => { if (isOpen) load(); }, [isOpen, load]);

  const chosen = useMemo(() => labels.filter(l => selected.has(l.payload)), [labels, selected]);
  const cells = useMemo(() => buildCells(chosen, { copies, skip }), [chosen, copies, skip]);
  const per = perSheet(layout);
  const sheetCount = Math.max(1, Math.ceil(cells.length / per));

  useEffect(() => {
    if (!isOpen || chosen.length === 0) { setPreviewHtml(''); return undefined; }
    let cancelled = false;
    const t = setTimeout(async () => {
      try {
        const html = await buildSheetDocument(cells.slice(0, per), layout, { cutGuides: settings.cutGuides });
        if (!cancelled) setPreviewHtml(html);
      } catch (e) {
        if (!cancelled) setError(e.message || 'Could not draw the labels');
      }
    }, 200);
    return () => { cancelled = true; clearTimeout(t); };
  }, [isOpen, cells, per, layout, settings.cutGuides, chosen.length]);

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

  const toggle = (key) => setSelected(prev => {
    const next = new Set(prev);
    if (next.has(key)) next.delete(key); else next.add(key);
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
        cutGuides: settings.cutGuides, autoPrint: true, title: title || 'Location labels',
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

  if (!isOpen) return null;

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
          <i className="fas fa-qrcode" style={{ color: '#3182ce', fontSize: '1.1rem' }} />
          <span>
            Location labels
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
            <Button variant="primary" onClick={handlePrint} disabled={chosen.length === 0 || printing}
              loading={printing} icon={<PrinterIcon size={16} />}>
              Print
            </Button>
          </div>
        </div>
      )}
    >
      <div style={{ display: 'grid', gridTemplateColumns: isMobile ? 'minmax(0, 1fr)' : '340px 1fr', gap: '20px', alignItems: 'start' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', minWidth: 0 }}>
          <div>
            <label style={label} htmlFor="loc-label-layout">Label size</label>
            <select id="loc-label-layout" value={settings.layout}
              onChange={e => updateSettings({ layout: e.target.value })} style={input}>
              {Object.values(LOCATION_LAYOUTS).map(l => (
                <option key={l.id} value={l.id}>{l.name} ({perSheet(l)}/A4)</option>
              ))}
            </select>
            <div style={{ fontSize: '0.75rem', color: '#718096', marginTop: '4px' }}>{layout.hint}</div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: '10px' }}>
            <div>
              <label style={label} htmlFor="loc-label-copies">Copies of each</label>
              <input id="loc-label-copies" type="number" min="1" max="10" step="1" value={copies}
                onChange={e => setCopies(Math.min(10, Math.max(1, parseInt(e.target.value, 10) || 1)))} style={input} />
            </div>
            <div>
              <label style={label} htmlFor="loc-label-skip" title="Leave already used cells of a sticker sheet empty">Skip first cells</label>
              <input id="loc-label-skip" type="number" min="0" max={per - 1} step="1" value={skip}
                onChange={e => setSkip(Math.min(per - 1, Math.max(0, parseInt(e.target.value, 10) || 0)))} style={input} />
            </div>
          </div>

          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer', fontSize: '0.85rem', color: '#2d3748' }}>
            <input type="checkbox" style={checkbox} checked={settings.cutGuides}
              onChange={e => updateSettings({ cutGuides: e.target.checked })} />
            Cut guides (dashed borders, for plain paper)
          </label>

          <div style={{ fontSize: '0.75rem', color: '#718096', background: '#f7fafc', borderRadius: '8px', padding: '8px 10px', lineHeight: 1.45 }}>
            In the print dialog: scale <b>100%</b> (Actual size), margins <b>None</b>.
            Stick the label where it is easy to scan: on the shelf edge, the cabinet door or the room door.
          </div>

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
              <span style={{ ...label, marginBottom: 0 }}>Places ({selected.size}/{labels.length})</span>
              <span style={{ display: 'flex', gap: '6px' }}>
                <button type="button" onClick={() => setSelected(new Set(labels.map(l => l.payload)))}
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
              {!loading && labels.length === 0 && (
                <div style={{ padding: '12px', color: '#718096', fontSize: '0.85rem' }}>No places to label.</div>
              )}
              {labels.map(l => (
                <label key={l.payload} style={{
                  display: 'flex', alignItems: 'center', gap: '8px', padding: '6px 10px',
                  borderBottom: '1px solid #f7fafc', fontSize: '0.8rem', cursor: 'pointer',
                }}>
                  <input type="checkbox" style={checkbox} checked={selected.has(l.payload)} onChange={() => toggle(l.payload)} />
                  <span style={{ fontSize: '0.68rem', fontWeight: 700, color: KIND_COLOR[l.kind], minWidth: '52px' }}>{(KIND[l.kind] || '').toUpperCase()}</span>
                  <span style={{ minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={l.path}>{l.path}</span>
                </label>
              ))}
            </div>
          </div>

          {error && (
            <div role="alert" style={{ color: '#c53030', background: '#fff5f5', border: '1px solid #fed7d7', borderRadius: '8px', padding: '8px 10px', fontSize: '0.85rem' }}>
              {error}
            </div>
          )}
        </div>

        <div ref={previewBox} style={{ minWidth: 0 }}>
          <div style={{ ...label, display: 'flex', justifyContent: 'space-between' }}>
            <span>Preview{sheetCount > 1 ? ' — sheet 1' : ''}</span>
          </div>
          <div style={{
            width: '100%', height: `${Math.round(1180 * previewScale)}px`, overflow: 'hidden',
            borderRadius: '8px', background: '#e2e8f0',
          }}>
            {previewHtml ? (
              <iframe title="Location label sheet preview" srcDoc={previewHtml} sandbox=""
                style={{
                  width: `${PREVIEW_WIDTH_PX}px`, height: '1180px', border: 0,
                  transform: `scale(${previewScale})`, transformOrigin: 'top left', pointerEvents: 'none',
                }} />
            ) : (
              <div style={{ padding: '24px', color: '#718096', fontSize: '0.85rem' }}>
                {chosen.length === 0 ? 'Select places to see the sheet.' : 'Drawing labels…'}
              </div>
            )}
          </div>
        </div>
      </div>
    </Modal>
  );
};

export default LocationLabelPrintModal;
