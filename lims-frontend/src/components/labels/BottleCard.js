// components/labels/BottleCard.js
// One bottle after a scan: contents, place, expiry, write-off, move, history.

import React, { useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import BottleHistory from './BottleHistory';

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);
const fmtDate = (v) => (v ? new Date(v).toLocaleDateString('ru-RU') : '—');

const isExpired = (c) => c.batch_status === 'expired'
  || (c.expiry_date && new Date(c.expiry_date).getTime() < Date.now());

const STATUS = {
  full: { text: 'Sealed', bg: '#c6f6d5', fg: '#22543d' },
  partial: { text: 'Opened', bg: '#fefcbf', fg: '#744210' },
  empty: { text: 'Empty', bg: '#fed7d7', fg: '#822727' },
  disposed: { text: 'Disposed', bg: '#e2e8f0', fg: '#4a5568' },
};

const ghsList = (value) => (value || '')
  .split(',').map(c => c.trim().toUpperCase()).filter(c => /^GHS0[1-9]$/.test(c));

const BottleCard = ({ c, canAct, onChanged, onMove, historyKey = 0 }) => {
  const [qty, setQty] = useState('');
  const [showHistory, setShowHistory] = useState(false);
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState(null);
  const expired = isExpired(c);
  const usable = c.container_status !== 'empty' && c.container_status !== 'disposed' && c.quantity > 0;
  const st = STATUS[c.container_status] || { text: c.container_status, bg: '#edf2f7', fg: '#2d3748' };
  const pct = c.original_quantity > 0 ? Math.max(0, Math.min(100, (c.quantity / c.original_quantity) * 100)) : 0;
  const location = c.position_name
    ? [c.room_name, c.zone_name, c.position_name].filter(Boolean).join(' / ')
    : null;

  const use = async () => {
    const n = parseFloat(String(qty).replace(',', '.'));
    if (!(n > 0)) { setMsg({ error: true, text: 'Enter an amount' }); return; }
    if (expired && !reason.trim()) { setMsg({ error: true, text: 'Expired batch: enter a reason' }); return; }
    setBusy(true);
    setMsg(null);
    try {
      const res = await api.useFromContainer(c.container_id, {
        quantity: n,
        source: 'scan',
        ...(expired ? { allow_expired: true, expired_reason: reason.trim() } : {}),
      });
      const d = res?.data || {};
      setQty('');
      setReason('');
      // Refresh first so the message and the bar change together
      await onChanged();
      setMsg({ text: `Used ${fmtQty(n)} ${c.unit} — ${fmtQty(d.container_remaining ?? 0)} ${c.unit} left in this bottle` });
    } catch (e) {
      setMsg({ error: true, text: e.message || 'Write-off failed' });
    } finally {
      setBusy(false);
    }
  };

  const row = { display: 'flex', justifyContent: 'space-between', gap: '12px', padding: '6px 0', borderBottom: '1px solid #f1f5f9', fontSize: '0.875rem' };
  const key = { color: '#718096', flexShrink: 0 };
  const val = { color: '#1a202c', fontWeight: 500, textAlign: 'right', overflowWrap: 'anywhere' };

  return (
    <div style={{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '14px', padding: '16px', boxShadow: '0 2px 10px rgba(26,54,93,0.06)' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '10px', alignItems: 'flex-start' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '1.15rem', fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere' }}>{c.reagent_name}</div>
          <div style={{ fontSize: '0.8rem', color: '#718096', marginTop: '2px' }}>
            {[c.cas_number && `CAS ${c.cas_number}`, c.formula].filter(Boolean).join(' · ') || ' '}
          </div>
        </div>
        <span style={{ background: st.bg, color: st.fg, fontWeight: 700, fontSize: '0.75rem', padding: '3px 10px', borderRadius: '999px', whiteSpace: 'nowrap' }}>
          {st.text}
        </span>
      </div>

      {ghsList(c.hazard_pictograms).length > 0 && (
        <div style={{ display: 'flex', gap: '6px', marginTop: '10px' }}>
          {ghsList(c.hazard_pictograms).map(g => (
            <img key={g} src={`/assets/ghs/${g}.svg`} alt={g} title={g} style={{ width: '34px', height: '34px' }} />
          ))}
        </div>
      )}

      <div style={{ marginTop: '12px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.875rem', marginBottom: '4px' }}>
          <span style={{ color: '#718096' }}>In this bottle</span>
          <span style={{ fontWeight: 700, color: '#1a202c' }}>{fmtQty(c.quantity)} / {fmtQty(c.original_quantity)} {c.unit}</span>
        </div>
        <div style={{ height: '6px', background: '#edf2f7', borderRadius: '3px', overflow: 'hidden' }}>
          <div style={{ width: `${pct}%`, height: '100%', background: pct <= 20 ? '#e53e3e' : '#38a169' }} />
        </div>
      </div>

      <div style={{ marginTop: '10px' }}>
        <div style={row}><span style={key}>Batch</span><span style={val}>{c.batch_number} · bottle #{c.sequence_number} of {c.batch_container_count}</span></div>
        <div style={row}><span style={key}>Location</span><span style={{ ...val, color: location ? '#1a202c' : '#a0aec0' }}>{location || 'Not placed'}</span></div>
        <div style={row}>
          <span style={key}>Expiry</span>
          <span style={{ ...val, color: expired ? '#c53030' : '#1a202c', fontWeight: expired ? 700 : 500 }}>
            {fmtDate(c.expiry_date)}{expired ? ' — expired' : ''}
          </span>
        </div>
        <div style={{ ...row, borderBottom: 'none' }}><span style={key}>Label code</span><span style={{ ...val, fontFamily: 'monospace' }}>{c.short_code}</span></div>
      </div>

      {canAct && usable && (
        <div style={{ marginTop: '12px', paddingTop: '12px', borderTop: '1px solid #e2e8f0', display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {expired && (
            <input
              type="text" value={reason} onChange={e => setReason(e.target.value)} maxLength={500}
              placeholder="Reason for using expired stock (required)"
              style={{ height: '44px', padding: '0 12px', border: '1px solid #fbd38d', borderRadius: '10px', background: '#fffaf0' }}
            />
          )}
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <div style={{ position: 'relative', flex: 1, minWidth: 0 }}>
              <input
                type="number" inputMode="decimal" step="any" min="0" value={qty}
                onChange={e => setQty(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') use(); }}
                placeholder={`Amount, max ${fmtQty(c.quantity)}`}
                aria-label={`Amount in ${c.unit}`}
                style={{ width: '100%', height: '44px', padding: '0 44px 0 12px', border: '1px solid #e2e8f0', borderRadius: '10px', boxSizing: 'border-box' }}
              />
              <span style={{ position: 'absolute', right: '12px', top: '50%', transform: 'translateY(-50%)', color: '#718096', fontWeight: 600, fontSize: '0.85rem', pointerEvents: 'none' }}>
                {c.unit}
              </span>
            </div>
            <Button variant="primary" onClick={use} disabled={busy} loading={busy}
              style={{ height: '44px', background: expired ? '#dd6b20' : '#38a169', flexShrink: 0 }}>
              Use
            </Button>
          </div>
          <Button variant="secondary" onClick={onMove} style={{ height: '44px' }}>
            {c.position_id ? 'Move to another shelf' : 'Place on a shelf'}
          </Button>
        </div>
      )}

      <div style={{ marginTop: '12px', borderTop: '1px solid #e2e8f0', paddingTop: '8px' }}>
        <button type="button" onClick={() => setShowHistory(v => !v)} aria-expanded={showHistory}
          style={{ width: '100%', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'none', border: 'none', padding: '6px 0', cursor: 'pointer', color: '#2b6cb0', fontWeight: 700, fontSize: '0.9rem' }}>
          <span>History of this bottle</span>
          <span aria-hidden="true">{showHistory ? '▴' : '▾'}</span>
        </button>
        {showHistory && <BottleHistory containerId={c.container_id} refreshKey={historyKey} />}
      </div>

      {msg && (
        <div role="status" style={{
          marginTop: '10px', padding: '8px 10px', borderRadius: '8px', fontSize: '0.85rem',
          background: msg.error ? '#fff5f5' : '#f0fff4', color: msg.error ? '#c53030' : '#276749',
          border: `1px solid ${msg.error ? '#fed7d7' : '#c6f6d5'}`,
        }}>
          {msg.error ? '⚠ ' : '✓ '}{msg.text}
        </div>
      )}
    </div>
  );
};

export default BottleCard;
