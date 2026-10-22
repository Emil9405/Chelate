// components/labels/LocationCard.js
// A scanned room, cabinet or shelf: where it is, what is on it.

import React from 'react';
import Button from '../Button';
import { KIND_LABEL, KIND_COLOR } from './codes';

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);

const LocationCard = ({ loc, canAct, onPutHere }) => {
  const color = KIND_COLOR[loc.kind] || '#4a5568';
  const isShelf = loc.kind === 'position';
  const childWord = loc.kind === 'room' ? 'Cabinets' : 'Shelves';

  return (
    <div style={{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '14px', padding: '16px', boxShadow: '0 2px 10px rgba(26,54,93,0.06)', position: 'relative', overflow: 'hidden' }}>
      <div aria-hidden="true" style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: '5px', background: loc.room_color || '#cbd5e0' }} />
      <div style={{ paddingLeft: '6px' }}>
        <span style={{ fontSize: '0.7rem', fontWeight: 800, letterSpacing: '0.06em', color, textTransform: 'uppercase' }}>
          {KIND_LABEL[loc.kind] || ''}
        </span>
        <div style={{ fontSize: '1.3rem', fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere', margin: '2px 0' }}>{loc.name}</div>
        <div style={{ fontSize: '0.85rem', color: '#718096', overflowWrap: 'anywhere' }}>{loc.path}</div>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', marginTop: '12px', padding: '8px 0', borderTop: '1px solid #f1f5f9', fontSize: '0.9rem' }}>
          <span style={{ color: '#718096' }}>Bottles recorded here</span>
          <span style={{ fontWeight: 800, color: '#1a202c' }}>{loc.bottle_count}</span>
        </div>
      </div>

      {canAct && isShelf && (
        <Button variant="primary" onClick={onPutHere} style={{ width: '100%', height: '46px', marginTop: '6px' }}>
          Put bottles here — scan them
        </Button>
      )}

      {loc.children && loc.children.length > 0 && (
        <div style={{ marginTop: '12px' }}>
          <div style={{ fontSize: '0.75rem', fontWeight: 700, color: '#4a5568', marginBottom: '4px' }}>{childWord} ({loc.children.length})</div>
          <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
            {loc.children.map(ch => (
              <li key={ch.id} style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', padding: '6px 0', borderBottom: '1px solid #f7fafc', fontSize: '0.875rem' }}>
                <span style={{ minWidth: 0, overflowWrap: 'anywhere' }}>{ch.name}</span>
                <span style={{ color: '#718096', whiteSpace: 'nowrap' }}>{ch.bottle_count} bottle{ch.bottle_count !== 1 ? 's' : ''}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {isShelf && (
        <div style={{ marginTop: '12px' }}>
          <div style={{ fontSize: '0.75rem', fontWeight: 700, color: '#4a5568', marginBottom: '4px' }}>On this shelf ({loc.bottles.length})</div>
          {loc.bottles.length === 0 ? (
            <div style={{ fontSize: '0.85rem', color: '#a0aec0' }}>Nothing is recorded on this shelf.</div>
          ) : (
            <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
              {loc.bottles.map(b => (
                <li key={b.container_id} style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', padding: '6px 0', borderBottom: '1px solid #f7fafc', fontSize: '0.85rem' }}>
                  <span style={{ minWidth: 0, overflowWrap: 'anywhere' }}>
                    <b>{b.reagent_name}</b> · {b.batch_number} #{b.sequence_number}
                    <span style={{ color: '#718096' }}> · {fmtQty(b.quantity)} {b.unit}</span>
                  </span>
                  <span style={{ fontFamily: 'monospace', color: '#a0aec0', whiteSpace: 'nowrap' }}>{b.short_code}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
};

export default LocationCard;
