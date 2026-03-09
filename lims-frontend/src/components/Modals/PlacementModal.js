// components/Reagents/PlacementModal.js
// Модалка размещения/перемещения контейнеров
//
// Place mode: распределяем unplaced контейнеры по полкам (multi-assignment)
// Move mode: выбираем СКОЛЬКО забрать с текущей позиции и КУДА переместить

import React, { useState, useEffect, useMemo } from 'react';
import LocationPicker from '../storage/LocationPicker';

// ==================== ICONS ====================

const PackageIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M16.5 9.4l-9-5.19M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/>
    <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
    <line x1="12" y1="22.08" x2="12" y2="12"/>
  </svg>
);

const MapPinIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0z" />
    <circle cx="12" cy="10" r="3" />
  </svg>
);

const CheckIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="20 6 9 17 4 12"/>
  </svg>
);

const PlusIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
  </svg>
);

const TrashIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
  </svg>
);

const AlertIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
  </svg>
);

// ==================== STEPPER ====================

const Stepper = ({ value, onChange, min = 1, max = 999, disabled = false, label }) => (
  <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
    {label && <span style={{ fontSize: '12px', fontWeight: '500', color: '#4a5568' }}>{label}</span>}
    <div style={{
      display: 'flex', alignItems: 'center',
      border: '1.5px solid #cbd5e0', borderRadius: '8px',
      overflow: 'hidden', height: '38px', opacity: disabled ? 0.5 : 1,
    }}>
      <button type="button"
        onClick={() => { if (value > min) onChange(value - 1); }}
        disabled={disabled || value <= min}
        style={{
          width: '36px', height: '100%', border: 'none',
          background: disabled || value <= min ? '#f7fafc' : '#edf2f7',
          cursor: disabled || value <= min ? 'not-allowed' : 'pointer',
          fontSize: '18px', fontWeight: '600',
          color: disabled || value <= min ? '#cbd5e0' : '#4a5568',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}
      >−</button>
      <div style={{
        minWidth: '48px', textAlign: 'center', fontWeight: '700', fontSize: '16px',
        color: '#1a202c', background: 'white', padding: '0 4px', fontVariantNumeric: 'tabular-nums',
      }}>{value}</div>
      <button type="button"
        onClick={() => { if (value < max) onChange(value + 1); }}
        disabled={disabled || value >= max}
        style={{
          width: '36px', height: '100%', border: 'none',
          background: disabled || value >= max ? '#f7fafc' : '#edf2f7',
          cursor: disabled || value >= max ? 'not-allowed' : 'pointer',
          fontSize: '18px', fontWeight: '600',
          color: disabled || value >= max ? '#cbd5e0' : '#4a5568',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}
      >+</button>
    </div>
  </div>
);

// ==================== ASSIGNMENT ROW (Place mode) ====================

const AssignmentRow = ({ index, assignment, maxAvailable, onChangeCount, onChangePosition, onRemove, disabled }) => {
  const totalMax = assignment.count + maxAvailable;
  return (
    <div style={{
      display: 'flex', alignItems: 'flex-start', gap: '12px',
      padding: '12px 14px', borderRadius: '10px',
      background: '#f7fafc', border: '1px solid #e2e8f0',
    }}>
      <Stepper value={assignment.count} onChange={(val) => onChangeCount(index, val)}
        min={1} max={totalMax} disabled={disabled} label="Count" />
      <div style={{ flex: 1, minWidth: '200px' }}>
        <span style={{ fontSize: '12px', fontWeight: '500', color: '#4a5568', display: 'block', marginBottom: '4px' }}>Destination</span>
        <LocationPicker value={assignment.positionId} onChange={(posId) => onChangePosition(index, posId)}
          showLabel={false} placeholder="Select shelf / position..." />
      </div>
      <button onClick={() => onRemove(index)} disabled={disabled} title="Remove"
        style={{ marginTop: '22px', width: '32px', height: '32px', display: 'flex', alignItems: 'center', justifyContent: 'center',
          borderRadius: '6px', border: '1px solid #e2e8f0', background: 'white', color: '#a0aec0', cursor: 'pointer', flexShrink: 0 }}
        onMouseEnter={e => { e.currentTarget.style.borderColor = '#fc8181'; e.currentTarget.style.color = '#e53e3e'; }}
        onMouseLeave={e => { e.currentTarget.style.borderColor = '#e2e8f0'; e.currentTarget.style.color = '#a0aec0'; }}
      ><TrashIcon size={14} /></button>
    </div>
  );
};

// ==================== CONTAINER SUMMARY ====================

const ContainerSummary = ({ containers, action }) => {
  const extractQty = (c) => {
    const q = c.quantity ?? c.container_quantity;
    return typeof q === 'object' && q !== null ? Number(q.parsedValue || 0) : Number(q || 0);
  };
  const totalQty = containers.reduce((s, c) => s + extractQty(c), 0);
  const cur = containers[0];
  const posLabel = cur?.room_name ? `${cur.room_name} / ${cur.zone_name || ''} / ${cur.position_name || ''}`.replace(/ \/ *$/g, '') : null;

  return (
    <div style={{
      padding: '14px 16px', borderRadius: '10px',
      background: 'linear-gradient(135deg, #ebf4ff 0%, #f0f5ff 100%)',
      border: '1px solid #bee3f8', display: 'flex', alignItems: 'center', gap: '16px', flexWrap: 'wrap',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <div style={{ width: '36px', height: '36px', borderRadius: '8px',
          background: action === 'move' ? '#fefcbf' : '#c6f6d5',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          border: action === 'move' ? '1px solid #ecc94b' : '1px solid #9ae6b4' }}>
          {action === 'move' ? '↗' : <PackageIcon size={18} color="#276749" />}
        </div>
        <div>
          <div style={{ fontWeight: '700', fontSize: '15px', color: '#1a365d' }}>
            {containers.length} container{containers.length !== 1 ? 's' : ''}
          </div>
          <div style={{ fontSize: '12px', color: '#4a5568' }}>Total: {totalQty.toFixed(1)} units</div>
        </div>
      </div>
      {action === 'move' && posLabel && (
        <div style={{ fontSize: '12px', padding: '4px 10px', borderRadius: '6px',
          background: '#f7fafc', border: '1px solid #e2e8f0', color: '#4a5568',
          display: 'flex', alignItems: 'center', gap: '4px' }}>
          <MapPinIcon size={12} color="#718096" /> From: {posLabel}
        </div>
      )}
    </div>
  );
};

// ==================== MOVE MODE ====================

const MoveContent = ({ containers, moveCount, setMoveCount, moveTarget, setMoveTarget, loading }) => {
  const total = containers.length;
  const remaining = total - moveCount;
  const cur = containers[0];
  const posLabel = cur?.room_name ? `${cur.room_name} / ${cur.position_name || ''}` : 'Current position';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr auto 1fr', gap: '12px', alignItems: 'center' }}>
        {/* FROM */}
        <div style={{ padding: '14px', borderRadius: '10px', background: '#f7fafc', border: '1px solid #e2e8f0', textAlign: 'center' }}>
          <div style={{ fontSize: '11px', fontWeight: '600', color: '#718096', textTransform: 'uppercase', marginBottom: '6px' }}>Stay at</div>
          <div style={{ fontSize: '12px', color: '#4a5568', marginBottom: '8px' }}>{posLabel}</div>
          <div style={{ fontSize: '28px', fontWeight: '800', color: remaining > 0 ? '#2d3748' : '#e53e3e', fontVariantNumeric: 'tabular-nums' }}>{remaining}</div>
          <div style={{ fontSize: '11px', color: '#a0aec0' }}>container{remaining !== 1 ? 's' : ''}</div>
        </div>

        {/* Arrow + Stepper */}
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
          <div style={{ fontSize: '20px', color: '#a0aec0' }}>→</div>
          <Stepper value={moveCount} onChange={setMoveCount} min={1} max={total} disabled={loading} label="Move" />
        </div>

        {/* TO */}
        <div style={{ padding: '14px', borderRadius: '10px',
          background: moveTarget ? '#f0fff4' : '#fffff0',
          border: moveTarget ? '1px solid #9ae6b4' : '1px solid #fbd38d', textAlign: 'center' }}>
          <div style={{ fontSize: '11px', fontWeight: '600', color: '#718096', textTransform: 'uppercase', marginBottom: '6px' }}>Move to</div>
          <div style={{ fontSize: '28px', fontWeight: '800', color: '#2d3748', fontVariantNumeric: 'tabular-nums' }}>{moveCount}</div>
          <div style={{ fontSize: '11px', color: '#a0aec0', marginBottom: '8px' }}>container{moveCount !== 1 ? 's' : ''}</div>
        </div>
      </div>

      <div>
        <span style={{ fontSize: '12px', fontWeight: '600', color: '#4a5568', display: 'block', marginBottom: '6px' }}>New location</span>
        <LocationPicker value={moveTarget} onChange={setMoveTarget} showLabel={false} placeholder="Select destination shelf..." />
      </div>
    </div>
  );
};

// ==================== PLACE MODE ====================

const PlaceContent = ({ containers, assignments, setAssignments, loading }) => {
  const totalAssigned = assignments.reduce((s, a) => s + a.count, 0);
  const remaining = containers.length - totalAssigned;

  const handleChangeCount = (idx, val) => setAssignments(prev => { const n = [...prev]; n[idx] = { ...n[idx], count: val }; return n; });
  const handleChangePosition = (idx, posId) => setAssignments(prev => { const n = [...prev]; n[idx] = { ...n[idx], positionId: posId }; return n; });
  const handleRemove = (idx) => { if (assignments.length > 1) setAssignments(prev => prev.filter((_, i) => i !== idx)); };
  const handleAdd = () => {
    if (remaining <= 0) {
      setAssignments(prev => {
        const n = [...prev];
        if (n.length > 0 && n[n.length - 1].count > 1) n[n.length - 1] = { ...n[n.length - 1], count: n[n.length - 1].count - 1 };
        return [...n, { count: 1, positionId: null }];
      });
    } else {
      setAssignments(prev => [...prev, { count: 1, positionId: null }]);
    }
  };

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
        <span style={{ fontSize: '13px', fontWeight: '600', color: '#2d3748' }}>Distribution</span>
        {remaining === 0 ? (
          <span style={{ fontSize: '12px', padding: '2px 8px', borderRadius: '4px', background: '#f0fff4', color: '#276749', border: '1px solid #c6f6d5', display: 'flex', alignItems: 'center', gap: '4px' }}>
            <CheckIcon size={12} color="#38a169" /> All assigned
          </span>
        ) : (
          <span style={{ fontSize: '12px', padding: '2px 8px', borderRadius: '4px',
            background: remaining > 0 ? '#fffaf0' : '#fff5f5', color: remaining > 0 ? '#c05621' : '#c53030',
            border: remaining > 0 ? '1px solid #fbd38d' : '1px solid #feb2b2' }}>
            {remaining > 0 ? `${remaining} unassigned` : `${Math.abs(remaining)} over-assigned`}
          </span>
        )}
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
        {assignments.map((a, idx) => {
          const othersTotal = assignments.reduce((s, x, i) => s + (i !== idx ? x.count : 0), 0);
          return <AssignmentRow key={idx} index={idx} assignment={a} maxAvailable={containers.length - othersTotal - a.count}
            onChangeCount={handleChangeCount} onChangePosition={handleChangePosition} onRemove={handleRemove} disabled={loading} />;
        })}
      </div>
      {containers.length > 1 && (
        <button onClick={handleAdd} disabled={loading}
          style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px',
            width: '100%', padding: '10px', marginTop: '10px',
            border: '1.5px dashed #cbd5e0', borderRadius: '10px', background: 'transparent',
            color: '#718096', cursor: 'pointer', fontSize: '13px', fontWeight: '500' }}
          onMouseEnter={e => { e.currentTarget.style.borderColor = '#4299e1'; e.currentTarget.style.color = '#2b6cb0'; e.currentTarget.style.background = '#ebf8ff'; }}
          onMouseLeave={e => { e.currentTarget.style.borderColor = '#cbd5e0'; e.currentTarget.style.color = '#718096'; e.currentTarget.style.background = 'transparent'; }}
        ><PlusIcon size={14} /> Add another shelf</button>
      )}
    </div>
  );
};

// ==================== MAIN MODAL ====================

const PlacementModal = ({ isOpen, containers, action, batchId, onConfirm, onClose }) => {
  const [assignments, setAssignments] = useState([{ count: containers.length, positionId: null }]);
  const [moveCount, setMoveCount] = useState(containers.length);
  const [moveTarget, setMoveTarget] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (containers.length > 0) {
      setAssignments([{ count: containers.length, positionId: null }]);
      setMoveCount(containers.length);
    }
  }, [containers.length]);

  const isValid = useMemo(() => {
    if (action === 'move') return moveCount > 0 && moveCount <= containers.length && !!moveTarget;
    const total = assignments.reduce((s, a) => s + a.count, 0);
    return total === containers.length && assignments.every(a => a.positionId && a.count > 0);
  }, [action, assignments, containers.length, moveCount, moveTarget]);

  const totalBtn = action === 'move' ? moveCount : assignments.reduce((s, a) => s + a.count, 0);

  const handleConfirm = async () => {
    if (!isValid) { setError(action === 'move' ? 'Select a destination' : 'Select a destination for each group'); return; }
    if (action === 'place') {
      const posIds = assignments.map(a => a.positionId);
      if (new Set(posIds).size !== posIds.length) { setError('Each group must go to a different position'); return; }
    }
    setLoading(true); setError('');
    try {
      if (action === 'move') {
        const toMove = containers.slice(0, moveCount);
        await onConfirm([{ containerIds: toMove.map(c => c.id), positionId: moveTarget }]);
      } else {
        const sorted = [...containers]; let offset = 0;
        const result = assignments.map(a => {
          const slice = sorted.slice(offset, offset + a.count); offset += a.count;
          return { containerIds: slice.map(c => c.id), positionId: a.positionId };
        });
        await onConfirm(result);
      }
    } catch (err) { setError(err.message || 'Failed'); } finally { setLoading(false); }
  };

  if (!isOpen) return null;

  return (
    <div style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0,0,0,0.45)',
      backdropFilter: 'blur(2px)', zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: '20px', animation: 'fadeIn 0.15s ease-out' }} onClick={onClose}>
      <div style={{ backgroundColor: '#fff', borderRadius: '14px', width: '100%',
        maxWidth: action === 'move' ? '560px' : '640px', maxHeight: '90vh', overflowY: 'auto',
        boxShadow: '0 25px 50px -12px rgba(0,0,0,0.25)', animation: 'slideUp 0.2s ease-out' }}
        onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div style={{ padding: '20px 24px 16px', borderBottom: '1px solid #edf2f7' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
            <div style={{ width: '32px', height: '32px', borderRadius: '8px',
              background: action === 'move' ? 'linear-gradient(135deg, #fefcbf, #fbd38d)' : 'linear-gradient(135deg, #c6f6d5, #9ae6b4)',
              display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '16px' }}>
              {action === 'move' ? '↗' : <MapPinIcon size={16} color="#276749" />}
            </div>
            <h3 style={{ margin: 0, fontSize: '18px', fontWeight: '700', color: '#1a202c' }}>
              {action === 'move' ? 'Move' : 'Place'} Containers
            </h3>
          </div>
          <p style={{ margin: '4px 0 0', fontSize: '13px', color: '#718096' }}>
            {action === 'move'
              ? 'Choose how many containers to move and select the new location.'
              : 'Assign containers to storage positions. Use the stepper to distribute across shelves.'}
          </p>
        </div>

        {/* Body */}
        <div style={{ padding: '16px 24px' }}>
          <ContainerSummary containers={containers} action={action} />
          <div style={{ marginTop: '20px' }}>
            {action === 'move' ? (
              <MoveContent containers={containers} moveCount={moveCount} setMoveCount={setMoveCount}
                moveTarget={moveTarget} setMoveTarget={setMoveTarget} loading={loading} />
            ) : (
              <PlaceContent containers={containers} assignments={assignments}
                setAssignments={setAssignments} loading={loading} />
            )}
          </div>
          {error && (
            <div style={{ marginTop: '12px', padding: '10px 14px', borderRadius: '8px',
              background: '#fff5f5', border: '1px solid #feb2b2', color: '#c53030', fontSize: '13px',
              display: 'flex', alignItems: 'center', gap: '8px' }}>
              <AlertIcon size={16} color="#c53030" /> {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{ padding: '16px 24px 20px', borderTop: '1px solid #edf2f7', display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
          <button onClick={onClose} disabled={loading}
            style={{ padding: '9px 20px', borderRadius: '8px', border: '1px solid #d1d5db',
              background: 'white', color: '#374151', fontWeight: '500', fontSize: '14px', cursor: 'pointer' }}>Cancel</button>
          <button onClick={handleConfirm} disabled={loading || !isValid}
            style={{ padding: '9px 24px', borderRadius: '8px', border: 'none',
              background: !isValid ? '#e2e8f0' : action === 'move' ? 'linear-gradient(135deg, #ecc94b, #d69e2e)' : 'linear-gradient(135deg, #48bb78, #38a169)',
              color: !isValid ? '#a0aec0' : 'white', fontWeight: '600', fontSize: '14px',
              cursor: !isValid || loading ? 'not-allowed' : 'pointer',
              display: 'flex', alignItems: 'center', gap: '6px', opacity: loading ? 0.7 : 1 }}>
            {loading ? (<><span style={{ width: '14px', height: '14px', border: '2px solid rgba(255,255,255,0.3)',
              borderTop: '2px solid white', borderRadius: '50%', animation: 'spin 0.6s linear infinite' }} />Processing...</>
            ) : (<><CheckIcon size={14} color="white" />{action === 'move' ? 'Move' : 'Place'} {totalBtn} container{totalBtn !== 1 ? 's' : ''}</>)}
          </button>
        </div>
      </div>
      <style>{`
        @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
        @keyframes slideUp { from { opacity: 0; transform: translateY(16px); } to { opacity: 1; transform: translateY(0); } }
        @keyframes spin { to { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
};

export default PlacementModal;
