// components/Reagents/PlacementModal.js
// Модалка размещения контейнеров с stepper для массового размещения по полкам
//
// Сценарий:
// 1. Пользователь выбирает контейнеры (или приходит с уже выбранными)
// 2. Степпером задаёт сколько штук разместить на выбранной полке
// 3. Может добавить несколько "назначений" (assignments) — разные полки
// 4. Нажимает "Confirm" — отправляется bulk запрос

import React, { useState, useEffect, useCallback, useMemo } from 'react';
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

const Stepper = ({ value, onChange, min = 1, max = 999, disabled = false, label }) => {
  const handleDecrement = () => { if (value > min) onChange(value - 1); };
  const handleIncrement = () => { if (value < max) onChange(value + 1); };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
      {label && (
        <span style={{ fontSize: '12px', fontWeight: '500', color: '#4a5568' }}>{label}</span>
      )}
      <div style={{
        display: 'flex', alignItems: 'center',
        border: '1.5px solid #cbd5e0', borderRadius: '8px',
        overflow: 'hidden', height: '38px',
        opacity: disabled ? 0.5 : 1,
        transition: 'border-color 0.15s',
      }}>
        <button
          type="button"
          onClick={handleDecrement}
          disabled={disabled || value <= min}
          style={{
            width: '36px', height: '100%', border: 'none',
            background: disabled || value <= min ? '#f7fafc' : '#edf2f7',
            cursor: disabled || value <= min ? 'not-allowed' : 'pointer',
            fontSize: '18px', fontWeight: '600',
            color: disabled || value <= min ? '#cbd5e0' : '#4a5568',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            transition: 'background 0.1s',
          }}
          onMouseEnter={e => { if (!disabled && value > min) e.target.style.background = '#e2e8f0'; }}
          onMouseLeave={e => { e.target.style.background = disabled || value <= min ? '#f7fafc' : '#edf2f7'; }}
        >
          −
        </button>
        <div style={{
          minWidth: '48px', textAlign: 'center',
          fontWeight: '700', fontSize: '16px',
          color: '#1a202c', background: 'white',
          padding: '0 4px',
          fontVariantNumeric: 'tabular-nums',
        }}>
          {value}
        </div>
        <button
          type="button"
          onClick={handleIncrement}
          disabled={disabled || value >= max}
          style={{
            width: '36px', height: '100%', border: 'none',
            background: disabled || value >= max ? '#f7fafc' : '#edf2f7',
            cursor: disabled || value >= max ? 'not-allowed' : 'pointer',
            fontSize: '18px', fontWeight: '600',
            color: disabled || value >= max ? '#cbd5e0' : '#4a5568',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            transition: 'background 0.1s',
          }}
          onMouseEnter={e => { if (!disabled && value < max) e.target.style.background = '#e2e8f0'; }}
          onMouseLeave={e => { e.target.style.background = disabled || value >= max ? '#f7fafc' : '#edf2f7'; }}
        >
          +
        </button>
      </div>
    </div>
  );
};

// ==================== ASSIGNMENT ROW ====================
// Одна строка назначения: "N контейнеров → полка X"

const AssignmentRow = ({
  index,
  assignment,         // { count, positionId, positionPath }
  maxAvailable,       // сколько ещё можно добавить (не считая текущую строку)
  onChangeCount,
  onChangePosition,
  onRemove,
  disabled,
}) => {
  const totalMax = assignment.count + maxAvailable;

  return (
    <div style={{
      display: 'flex', alignItems: 'flex-start', gap: '12px',
      padding: '12px 14px', borderRadius: '10px',
      background: '#f7fafc', border: '1px solid #e2e8f0',
      transition: 'box-shadow 0.15s',
    }}>
      {/* Stepper */}
      <Stepper
        value={assignment.count}
        onChange={(val) => onChangeCount(index, val)}
        min={1}
        max={totalMax}
        disabled={disabled}
        label="Count"
      />

      {/* Location Picker */}
      <div style={{ flex: 1, minWidth: '200px' }}>
        <span style={{ fontSize: '12px', fontWeight: '500', color: '#4a5568', display: 'block', marginBottom: '4px' }}>
          Destination
        </span>
        <LocationPicker
          value={assignment.positionId}
          onChange={(posId) => onChangePosition(index, posId)}
          showLabel={false}
          placeholder="Select shelf / position..."
        />
      </div>

      {/* Remove button */}
      <button
        onClick={() => onRemove(index)}
        disabled={disabled}
        title="Remove assignment"
        style={{
          marginTop: '22px',
          width: '32px', height: '32px',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          borderRadius: '6px', border: '1px solid #e2e8f0',
          background: 'white', color: '#a0aec0', cursor: 'pointer',
          transition: 'all 0.15s', flexShrink: 0,
        }}
        onMouseEnter={e => { e.currentTarget.style.borderColor = '#fc8181'; e.currentTarget.style.color = '#e53e3e'; e.currentTarget.style.background = '#fff5f5'; }}
        onMouseLeave={e => { e.currentTarget.style.borderColor = '#e2e8f0'; e.currentTarget.style.color = '#a0aec0'; e.currentTarget.style.background = 'white'; }}
      >
        <TrashIcon size={14} />
      </button>
    </div>
  );
};

// ==================== CONTAINER SUMMARY ====================

const ContainerSummary = ({ containers, action }) => {
  const placed = containers.filter(c => c.position_id || c.placement_id);
  const unplaced = containers.filter(c => !c.position_id && !c.placement_id);

  const extractQty = (c) => {
    const q = c.quantity ?? c.container_quantity;
    return typeof q === 'object' && q !== null ? Number(q.parsedValue || 0) : Number(q || 0);
  };

  const totalQty = containers.reduce((s, c) => s + extractQty(c), 0);

  return (
    <div style={{
      padding: '14px 16px', borderRadius: '10px',
      background: 'linear-gradient(135deg, #ebf4ff 0%, #f0f5ff 100%)',
      border: '1px solid #bee3f8',
      display: 'flex', alignItems: 'center', gap: '16px',
      flexWrap: 'wrap',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <div style={{
          width: '36px', height: '36px', borderRadius: '8px',
          background: action === 'move' ? '#fefcbf' : '#c6f6d5',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          border: action === 'move' ? '1px solid #ecc94b' : '1px solid #9ae6b4',
        }}>
          {action === 'move' ? '↗' : <PackageIcon size={18} color={action === 'move' ? '#b7791f' : '#276749'} />}
        </div>
        <div>
          <div style={{ fontWeight: '700', fontSize: '15px', color: '#1a365d' }}>
            {containers.length} container{containers.length !== 1 ? 's' : ''}
          </div>
          <div style={{ fontSize: '12px', color: '#4a5568' }}>
            Total: {totalQty.toFixed(1)} units
          </div>
        </div>
      </div>

      {action === 'place' && unplaced.length > 0 && unplaced.length < containers.length && (
        <div style={{
          fontSize: '12px', padding: '4px 10px', borderRadius: '6px',
          background: '#fffaf0', border: '1px solid #fbd38d', color: '#744210',
        }}>
          {unplaced.length} unplaced · {placed.length} already placed
        </div>
      )}

      {action === 'move' && placed.length > 0 && (
        <div style={{
          fontSize: '12px', padding: '4px 10px', borderRadius: '6px',
          background: '#f7fafc', border: '1px solid #e2e8f0', color: '#4a5568',
          display: 'flex', alignItems: 'center', gap: '4px',
        }}>
          <MapPinIcon size={12} color="#718096" />
          Currently at: {placed[0].room_name} / {placed[0].position_name}
          {placed.length > 1 && ` (+${placed.length - 1} more)`}
        </div>
      )}
    </div>
  );
};

// ==================== MAIN MODAL ====================

const PlacementModal = ({
  isOpen,
  containers,           // массив контейнеров для размещения
  action,               // 'place' | 'move'
  batchId,
  onConfirm,            // async (assignments: [{ containerIds: [], positionId }]) => void
  onClose,
}) => {
  // Assignments: распределение контейнеров по полкам
  // [{ count: N, positionId: 'xxx', positionPath: null }]
  const [assignments, setAssignments] = useState([
    { count: containers.length, positionId: null }
  ]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Пересчитываем, когда меняется кол-во контейнеров
  useEffect(() => {
    if (containers.length > 0) {
      setAssignments([{ count: containers.length, positionId: null }]);
    }
  }, [containers.length]);

  // Сколько контейнеров ещё не распределено
  const totalAssigned = useMemo(
    () => assignments.reduce((s, a) => s + a.count, 0),
    [assignments]
  );
  const remaining = containers.length - totalAssigned;

  // Проверяем валидность
 // Проверяем валидность
  const isValid = useMemo(() => {
    
    if (totalAssigned > containers.length) return false;
    if (action === 'place' && totalAssigned !== containers.length) return false;
    if (action === 'move' && totalAssigned === 0) return false;
    // У каждого назначения должна быть выбрана полка и количество > 0
    return assignments.every(a => a.positionId && a.count > 0);
  }, [assignments, totalAssigned, containers.length, action]);

  // --- Handlers ---

  const handleChangeCount = (idx, newCount) => {
    setAssignments(prev => {
      const next = [...prev];
      next[idx] = { ...next[idx], count: newCount };
      return next;
    });
    setError('');
  };

  const handleChangePosition = (idx, positionId) => {
    setAssignments(prev => {
      const next = [...prev];
      next[idx] = { ...next[idx], positionId };
      return next;
    });
    setError('');
  };

  const handleRemoveAssignment = (idx) => {
    if (assignments.length <= 1) return;
    setAssignments(prev => prev.filter((_, i) => i !== idx));
  };

  const handleAddAssignment = () => {
    if (remaining <= 0) {
      // Перераспределяем: забираем 1 из последней строки
      setAssignments(prev => {
        const next = [...prev];
        if (next.length > 0 && next[next.length - 1].count > 1) {
          next[next.length - 1] = { ...next[next.length - 1], count: next[next.length - 1].count - 1 };
        }
        return [...next, { count: 1, positionId: null }];
      });
    } else {
      setAssignments(prev => [...prev, { count: Math.min(remaining, 1), positionId: null }]);
    }
  };

  const handleConfirm = async () => {
    // Проверки
    if (!isValid) {
      setError('Select a destination for each group of containers');
      return;
    }

    // Проверяем дубликаты позиций
    const posIds = assignments.map(a => a.positionId);
    const uniquePos = new Set(posIds);
    if (uniquePos.size !== posIds.length) {
      setError('Each group must go to a different position. Merge groups going to the same shelf.');
      return;
    }

    setLoading(true);
    setError('');

    try {
      // Собираем назначения: разбиваем контейнеры по assignments
      const sortedContainers = [...containers];
      let offset = 0;
      const result = assignments.map(a => {
        const slice = sortedContainers.slice(offset, offset + a.count);
        offset += a.count;
        return {
          containerIds: slice.map(c => c.id),
          positionId: a.positionId,
        };
      });

      await onConfirm(result);
    } catch (err) {
      setError(err.message || 'Failed to place containers');
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div
      style={{
        position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.45)',
        backdropFilter: 'blur(2px)',
        zIndex: 9999,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        padding: '20px',
        animation: 'fadeIn 0.15s ease-out',
      }}
      onClick={onClose}
    >
      <div
        style={{
          backgroundColor: '#fff', borderRadius: '14px',
          width: '100%', maxWidth: '640px',
          maxHeight: '90vh', overflowY: 'auto',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
          animation: 'slideUp 0.2s ease-out',
        }}
        onClick={e => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{
          padding: '20px 24px 16px',
          borderBottom: '1px solid #edf2f7',
        }}>
          <div style={{
            display: 'flex', alignItems: 'center', gap: '10px',
            marginBottom: '4px',
          }}>
            <div style={{
              width: '32px', height: '32px', borderRadius: '8px',
              background: action === 'move'
                ? 'linear-gradient(135deg, #fefcbf, #fbd38d)'
                : 'linear-gradient(135deg, #c6f6d5, #9ae6b4)',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              fontSize: '16px',
            }}>
              {action === 'move' ? '↗' : <MapPinIcon size={16} color="#276749" />}
            </div>
            <h3 style={{ margin: 0, fontSize: '18px', fontWeight: '700', color: '#1a202c' }}>
              {action === 'move' ? 'Move' : 'Place'} Containers
            </h3>
          </div>
          <p style={{ margin: '4px 0 0', fontSize: '13px', color: '#718096' }}>
            {action === 'move'
              ? 'Select new destination(s) for the containers'
              : 'Assign containers to storage positions. Use the stepper to distribute across multiple shelves.'}
          </p>
        </div>

        {/* Body */}
        <div style={{ padding: '16px 24px' }}>
          {/* Container summary */}
          <ContainerSummary containers={containers} action={action} />

          {/* Assignments */}
          <div style={{ marginTop: '20px' }}>
            <div style={{
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              marginBottom: '12px',
            }}>
              <span style={{ fontSize: '13px', fontWeight: '600', color: '#2d3748' }}>
                Distribution
              </span>
              
              {/* Предупреждение для PLACE: если остались нераспределенные */}
              {remaining !== 0 && action === 'place' && (
                <span style={{
                  fontSize: '12px', padding: '2px 8px', borderRadius: '4px',
                  background: remaining > 0 ? '#fffaf0' : '#fff5f5',
                  color: remaining > 0 ? '#c05621' : '#c53030',
                  border: remaining > 0 ? '1px solid #fbd38d' : '1px solid #feb2b2',
                }}>
                  {remaining > 0 ? `${remaining} unassigned` : `${Math.abs(remaining)} over-assigned`}
                </span>
              )}

              {/* Инфо для MOVE: сколько контейнеров останется на текущей полке */}
              {remaining > 0 && action === 'move' && (
                <span style={{
                  fontSize: '12px', padding: '2px 8px', borderRadius: '4px',
                  background: '#ebf8ff', color: '#2b6cb0', border: '1px solid #bee3f8',
                }}>
                  {remaining} staying at current location
                </span>
              )}

              {/* Ошибка для MOVE: если случайно накликали больше, чем есть */}
              {remaining < 0 && action === 'move' && (
                <span style={{
                  fontSize: '12px', padding: '2px 8px', borderRadius: '4px',
                  background: '#fff5f5', color: '#c53030', border: '1px solid #feb2b2',
                }}>
                  {Math.abs(remaining)} over-assigned
                </span>
              )}

              {/* Успех: когда распределили ровно всё */}
              {remaining === 0 && (
                <span style={{
                  fontSize: '12px', padding: '2px 8px', borderRadius: '4px',
                  background: '#f0fff4', color: '#276749', border: '1px solid #c6f6d5',
                  display: 'flex', alignItems: 'center', gap: '4px',
                }}>
                  <CheckIcon size={12} color="#38a169" /> All assigned
                </span>
              )}
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
              {assignments.map((a, idx) => {
                // maxAvailable = сколько свободных + те что уже в этой строке вычитаем из суммы
                const othersTotal = assignments.reduce((s, x, i) => s + (i !== idx ? x.count : 0), 0);
                const maxForThis = containers.length - othersTotal;
                return (
                  <AssignmentRow
                    key={idx}
                    index={idx}
                    assignment={a}
                    maxAvailable={maxForThis - a.count}
                    onChangeCount={handleChangeCount}
                    onChangePosition={handleChangePosition}
                    onRemove={handleRemoveAssignment}
                    disabled={loading}
                  />
                );
              })}
            </div>

            {/* Add assignment button */}
            {containers.length > 1 && (
              <button
                onClick={handleAddAssignment}
                disabled={loading}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px',
                  width: '100%', padding: '10px',
                  marginTop: '10px',
                  border: '1.5px dashed #cbd5e0', borderRadius: '10px',
                  background: 'transparent', color: '#718096',
                  cursor: 'pointer', fontSize: '13px', fontWeight: '500',
                  transition: 'all 0.15s',
                }}
                onMouseEnter={e => { e.currentTarget.style.borderColor = '#4299e1'; e.currentTarget.style.color = '#2b6cb0'; e.currentTarget.style.background = '#ebf8ff'; }}
                onMouseLeave={e => { e.currentTarget.style.borderColor = '#cbd5e0'; e.currentTarget.style.color = '#718096'; e.currentTarget.style.background = 'transparent'; }}
              >
                <PlusIcon size={14} />
                Add another shelf
              </button>
            )}
          </div>

          {/* Error */}
          {error && (
            <div style={{
              marginTop: '12px', padding: '10px 14px', borderRadius: '8px',
              background: '#fff5f5', border: '1px solid #feb2b2',
              color: '#c53030', fontSize: '13px',
              display: 'flex', alignItems: 'center', gap: '8px',
            }}>
              <AlertIcon size={16} color="#c53030" />
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{
          padding: '16px 24px 20px',
          borderTop: '1px solid #edf2f7',
          display: 'flex', justifyContent: 'flex-end', gap: '10px',
        }}>
          <button
            onClick={onClose}
            disabled={loading}
            style={{
              padding: '9px 20px', borderRadius: '8px',
              border: '1px solid #d1d5db', background: 'white',
              color: '#374151', fontWeight: '500', fontSize: '14px',
              cursor: 'pointer', transition: 'all 0.15s',
            }}
            onMouseEnter={e => { e.currentTarget.style.background = '#f9fafb'; }}
            onMouseLeave={e => { e.currentTarget.style.background = 'white'; }}
          >
            Cancel
          </button>
          <button
            onClick={handleConfirm}
            disabled={loading || !isValid}
            style={{
              padding: '9px 24px', borderRadius: '8px',
              border: 'none',
              background: !isValid
                ? '#e2e8f0'
                : action === 'move'
                  ? 'linear-gradient(135deg, #ecc94b, #d69e2e)'
                  : 'linear-gradient(135deg, #48bb78, #38a169)',
              color: !isValid ? '#a0aec0' : 'white',
              fontWeight: '600', fontSize: '14px',
              cursor: !isValid || loading ? 'not-allowed' : 'pointer',
              transition: 'all 0.15s',
              display: 'flex', alignItems: 'center', gap: '6px',
              opacity: loading ? 0.7 : 1,
            }}
          >
            {loading ? (
              <>
                <span style={{
                  width: '14px', height: '14px', border: '2px solid rgba(255,255,255,0.3)',
                  borderTop: '2px solid white', borderRadius: '50%',
                  animation: 'spin 0.6s linear infinite',
                }} />
                Processing...
              </>
            ) : (
              <>
                <CheckIcon size={14} color="white" />
                {action === 'move' ? 'Move' : 'Place'} {totalAssigned} container{totalAssigned !== 1 ? 's' : ''}
              </>
            )}
          </button>
        </div>
      </div>

      {/* Animations */}
      <style>{`
        @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
        @keyframes slideUp { from { opacity: 0; transform: translateY(16px); } to { opacity: 1; transform: translateY(0); } }
        @keyframes spin { to { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
};

export default PlacementModal;
