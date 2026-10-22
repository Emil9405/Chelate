// components/labels/BottleHistory.js
// Timeline of one bottle: placements, moves, draws, inventory sightings.

import React, { useEffect, useState } from 'react';
import { api } from '../../services/api';

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);

const fmtWhen = (v) => {
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleString('ru-RU', {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit',
  });
};

const SOURCE_LABEL = {
  scan: 'scan', inventory: 'inventory', bulk: 'bulk', experiment: 'experiment',
  import: 'import', system: 'system',
};

const STYLE = {
  placed: { icon: '📍', color: '#2b6cb0' },
  moved: { icon: '↗', color: '#2b6cb0' },
  unplaced: { icon: '⤫', color: '#718096' },
  opened: { icon: '⊙', color: '#b7791f' },
  used: { icon: '−', color: '#2f855a' },
  emptied: { icon: '∅', color: '#c53030' },
  disposed: { icon: '🗑', color: '#4a5568' },
  seen: { icon: '✓', color: '#2f855a' },
  missing: { icon: '?', color: '#c53030' },
};

/** One line of text for an event */
export const describeEvent = (e) => {
  const unit = e.unit || '';
  switch (e.event_type) {
    case 'placed': return `Placed on ${e.to_location || 'a shelf'}`;
    case 'moved': return `Moved ${e.from_location || '?'} → ${e.to_location || '?'}`;
    case 'unplaced': return `Removed from ${e.from_location || 'its shelf'}`;
    case 'opened': return 'Opened';
    case 'used': {
      const left = e.quantity_after != null ? ` — ${fmtQty(e.quantity_after)} ${unit} left` : '';
      return `Used ${fmtQty(e.quantity)} ${unit}${left}`;
    }
    case 'emptied': return 'Emptied';
    case 'disposed': return `Disposed${e.from_location ? ` (was at ${e.from_location})` : ''}`;
    case 'seen': return `Checked in inventory at ${e.to_location || '?'}`;
    case 'missing': return `Not found in inventory${e.session_scope ? ` of ${e.session_scope}` : ''}`;
    default: return e.event_type;
  }
};

const BottleHistory = ({ containerId, refreshKey = 0, limit = 50 }) => {
  const [events, setEvents] = useState(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let cancelled = false;
    setError('');
    api.getContainerHistory(containerId)
      .then(rows => { if (!cancelled) setEvents(rows); })
      .catch(e => { if (!cancelled) setError(e.message || 'Could not load history'); });
    return () => { cancelled = true; };
  }, [containerId, refreshKey]);

  if (error) return <div style={{ color: '#c53030', fontSize: '0.85rem' }}>{error}</div>;
  if (!events) return <div style={{ color: '#a0aec0', fontSize: '0.85rem' }}>Loading history…</div>;
  if (events.length === 0) return <div style={{ color: '#a0aec0', fontSize: '0.85rem' }}>No history recorded yet.</div>;

  return (
    <ol style={{ listStyle: 'none', margin: 0, padding: 0 }} aria-label="Bottle history">
      {events.slice(0, limit).map((e) => {
        const st = STYLE[e.event_type] || { icon: '•', color: '#4a5568' };
        const purpose = e.event_type === 'used' && e.note ? e.note : null;
        const note = e.event_type === 'seen' && e.note && e.note.includes('recorded') ? e.note.split('; ').slice(1).join('; ') : null;
        return (
          <li key={e.id} style={{ display: 'flex', gap: '10px', padding: '7px 0', borderBottom: '1px solid #f1f5f9' }}>
            <span aria-hidden="true" style={{
              flex: '0 0 24px', height: '24px', borderRadius: '12px', background: '#f7fafc',
              color: st.color, display: 'flex', alignItems: 'center', justifyContent: 'center',
              fontWeight: 800, fontSize: '0.8rem',
            }}>{st.icon}</span>
            <div style={{ minWidth: 0, flex: 1 }}>
              <div style={{ fontSize: '0.85rem', color: '#1a202c', overflowWrap: 'anywhere' }}>{describeEvent(e)}</div>
              {(purpose || note) && (
                <div style={{ fontSize: '0.78rem', color: '#718096', overflowWrap: 'anywhere' }}>{purpose || note}</div>
              )}
              <div style={{ fontSize: '0.75rem', color: '#a0aec0' }}>
                {fmtWhen(e.created_at)}{e.username ? ` · ${e.username}` : ''}
                {SOURCE_LABEL[e.source] && (
                  <span style={{ marginLeft: '6px', padding: '0 6px', borderRadius: '8px', background: '#edf2f7', color: '#4a5568' }}>
                    {SOURCE_LABEL[e.source]}
                  </span>
                )}
              </div>
            </div>
          </li>
        );
      })}
    </ol>
  );
};

export default BottleHistory;
