// components/inventory/InventoryPanel.js
// Storage → Inventory tab: start an inventory of a room, cabinet or shelf,
// continue an open one, or open the report of a finished one.

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import InventorySession from './InventorySession';
import { canActOnStock } from '../labels/permissions';

const fmtWhen = (v) => {
  if (!v) return '';
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleString('ru-RU', {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit',
  });
};

const STATUS = {
  open: { label: 'In progress', bg: '#bee3f8', fg: '#2a4365' },
  closed: { label: 'Finished', bg: '#c6f6d5', fg: '#22543d' },
  cancelled: { label: 'Cancelled', bg: '#e2e8f0', fg: '#4a5568' },
};

const InventoryList = ({ hierarchy, user, onOpen }) => {
  const canAct = canActOnStock(user);
  const [sessions, setSessions] = useState(null);
  const [scope, setScope] = useState('');
  const [error, setError] = useState('');
  const [starting, setStarting] = useState(false);

  const load = useCallback(async () => {
    try {
      setSessions(await api.listInventorySessions('all'));
    } catch (e) {
      setError(e.message || 'Could not load inventory checks');
      setSessions([]);
    }
  }, []);
  useEffect(() => { load(); }, [load]);

  // "room:<id>", "zone:<id>", "position:<id>"
  const groups = useMemo(() => (hierarchy || []).map(room => ({
    room,
    options: [
      { value: `room:${room.id}`, label: `${room.name} — whole room` },
      ...(room.zones || []).flatMap(z => [
        { value: `zone:${z.id}`, label: `${room.name} / ${z.name}` },
        ...(z.positions || []).map(p => ({ value: `position:${p.id}`, label: `${room.name} / ${z.name} / ${p.name}` })),
      ]),
    ],
  })), [hierarchy]);

  const startNew = async () => {
    if (!scope) return;
    const [type, ...rest] = scope.split(':');
    setStarting(true);
    setError('');
    try {
      const r = await api.startInventory(type, rest.join(':'));
      onOpen(r.session.id);
    } catch (e) {
      setError(e.message || 'Could not start the inventory check');
    } finally {
      setStarting(false);
    }
  };

  const card = { background: 'white', border: '1px solid #e5e7eb', borderRadius: '12px', padding: '14px' };

  return (
    <div>
      {canAct && (
        <div style={{ ...card, marginBottom: '16px' }}>
          <div style={{ fontWeight: 700, color: '#1f2937', marginBottom: '4px' }}>New inventory check</div>
          <div style={{ fontSize: '0.82rem', color: '#6b7280', marginBottom: '10px' }}>
            Choose the place to check. You will scan every bottle there; whatever Chelate records
            in that place but you do not scan is listed as missing.
          </div>
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <select aria-label="Place to check" value={scope} onChange={e => setScope(e.target.value)}
              style={{ flex: '1 1 260px', minWidth: 0, height: '42px', padding: '0 10px', border: '1px solid #d1d5db', borderRadius: '8px', background: 'white' }}>
              <option value="">Select a room, cabinet or shelf…</option>
              {groups.map(g => (
                <optgroup key={g.room.id} label={g.room.name}>
                  {g.options.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
                </optgroup>
              ))}
            </select>
            <Button variant="primary" onClick={startNew} disabled={!scope || starting} loading={starting}>Start</Button>
          </div>
        </div>
      )}

      {error && <div role="alert" style={{ color: '#c53030', marginBottom: '10px' }}>{error}</div>}

      {sessions === null ? (
        <div style={{ color: '#9ca3af' }}>Loading…</div>
      ) : sessions.length === 0 ? (
        <div style={{ ...card, color: '#6b7280', textAlign: 'center' }}>No inventory checks yet.</div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
          {sessions.map(s => {
            const st = STATUS[s.status] || STATUS.cancelled;
            return (
              <button key={s.id} type="button" onClick={() => onOpen(s.id)}
                style={{ ...card, textAlign: 'left', cursor: 'pointer', display: 'block', width: '100%' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', alignItems: 'flex-start' }}>
                  <div style={{ fontWeight: 700, color: '#1f2937', overflowWrap: 'anywhere' }}>{s.scope_name}</div>
                  <span style={{ background: st.bg, color: st.fg, fontWeight: 700, fontSize: '0.72rem', padding: '2px 8px', borderRadius: '999px', whiteSpace: 'nowrap' }}>
                    {st.label}
                  </span>
                </div>
                <div style={{ fontSize: '0.8rem', color: '#6b7280', marginTop: '2px' }}>
                  {fmtWhen(s.started_at)}{s.started_by_name ? ` · ${s.started_by_name}` : ''}
                </div>
                <div style={{ fontSize: '0.82rem', color: '#374151', marginTop: '6px' }}>
                  {s.status === 'open' && <>{s.scanned_count} scanned so far — tap to continue</>}
                  {s.status === 'closed' && (
                    <>
                      Found <b>{s.found_count}</b> of {s.expected_count}
                      {' · '}missing <b style={{ color: s.missing_count ? '#c53030' : undefined }}>{s.missing_count}</b>
                      {' · '}misplaced <b style={{ color: s.misplaced_count ? '#c05621' : undefined }}>{s.misplaced_count}</b>
                    </>
                  )}
                  {s.status === 'cancelled' && <>Discarded</>}
                </div>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};

const InventoryPanel = ({ hierarchy, user, activeId, setActiveId }) => (
  activeId
    ? <InventorySession key={activeId} id={activeId} user={user} onBack={() => setActiveId(null)} />
    : <InventoryList hierarchy={hierarchy} user={user} onOpen={setActiveId} />
);

export default InventoryPanel;
