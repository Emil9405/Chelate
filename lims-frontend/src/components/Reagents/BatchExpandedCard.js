// components/Reagents/BatchExpandedCard.js
// Expanded batch card: Details | Grouped Containers | Usage bar

import React, { useEffect } from 'react';
import Button from '../Button';
import { UseIcon } from './icons';
import {
  EditIcon,
  TrashIcon,
  ClockIcon,
} from '../Icons';

// ==================== HELPERS ====================

const extractQty = (container) => {
  if (!container) return 0;
  const q = container.quantity ?? container.container_quantity;
  return typeof q === 'object' && q !== null ? Number(q.parsedValue || 0) : Number(q || 0);
};

const getStatus = (c) => c.container_status ?? c.status ?? 'full';

// ==================== DETAIL ROW ====================

const DetailRow = ({ label, value, color }) => (
  <div style={{ display: 'flex', justifyContent: 'space-between', padding: '4px 0', borderBottom: '1px solid #f7fafc' }}>
    <span style={{ color: '#718096', fontSize: '0.8rem' }}>{label}</span>
    <span style={{ color: color || '#1a365d', fontSize: '0.8rem', fontWeight: '500', textAlign: 'right', maxWidth: '60%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
      {value || '—'}
    </span>
  </div>
);

// ==================== GROUPING LOGIC ====================

const groupContainers = (containers) => {
  const map = new Map();

  for (const c of containers) {
    const posId = c.position_id || '__unplaced__';
    const status = getStatus(c);
    const opened = c.is_opened ? 'opened' : 'sealed';
    const key = `${posId}|${status}|${opened}`;

    if (!map.has(key)) {
      map.set(key, {
        key,
        positionId: c.position_id || null,
        positionName: c.position_name || null,
        roomName: c.room_name || null,
        roomColor: c.room_color || null,
        zoneName: c.zone_name || null,
        status,
        isOpened: c.is_opened,
        count: 0,
        totalQty: 0,
        containers: [],
      });
    }

    const group = map.get(key);
    group.count += 1;
    group.totalQty += extractQty(c);
    group.containers.push(c);
  }

  return Array.from(map.values()).sort((a, b) => {
    if (a.positionId && !b.positionId) return -1;
    if (!a.positionId && b.positionId) return 1;
    if (a.isOpened && !b.isOpened) return -1;
    if (!a.isOpened && b.isOpened) return 1;
    return (a.roomName || '').localeCompare(b.roomName || '');
  });
};

// ==================== GROUP STATUS BADGE ====================

const GroupBadge = ({ status, isOpened }) => {
  const colors = {
    full: { bg: '#c6f6d5', text: '#22543d' },
    partial: { bg: '#fefcbf', text: '#744210' },
    empty: { bg: '#fed7d7', text: '#822727' },
    disposed: { bg: '#e2e8f0', text: '#718096' },
  };
  const c = colors[status] || colors.full;
  const label = isOpened ? 'Opened' : 'Sealed';

  return (
    <span style={{
      fontSize: '11px', padding: '2px 8px', borderRadius: '4px',
      background: c.bg, color: c.text, fontWeight: '600',
      display: 'inline-flex', alignItems: 'center', gap: '4px',
    }}>
      {isOpened ? '⊙' : '🔒'} {label}
    </span>
  );
};

// ==================== GROUPED CONTAINER LIST ====================

const GroupedContainerList = ({ batch, containers, loading, onSplit, onRefresh, onPlaceContainer, onMoveContainer }) => {
  if (loading) {
    return <div style={{ fontSize: '0.8rem', color: '#a0aec0', padding: '8px 0' }}>Loading containers...</div>;
  }

  if (!containers || containers.length === 0) {
    return (
      <div style={{ padding: '8px 0' }}>
        <div style={{ fontSize: '0.8rem', color: '#a0aec0', marginBottom: '8px' }}>
          No containers yet.
        </div>
        {batch.pack_size && batch.pack_size > 0 && (
          <Button
            size="small"
            variant="secondary"
            onClick={() => onSplit(batch.id, batch.pack_size)}
            style={{ fontSize: '11px' }}
          >
            Split into {Math.ceil(batch.quantity / batch.pack_size)} containers ({batch.pack_size} {batch.unit} each)
          </Button>
        )}
      </div>
    );
  }

  const groups = groupContainers(containers);
  const allUnplaced = containers.filter(c => !c.position_id && !c.placement_id && getStatus(c) !== 'disposed');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
      {/* Place All Unplaced button */}
      {allUnplaced.length > 1 && onPlaceContainer && (
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          padding: '6px 10px', borderRadius: '8px',
          background: '#fffff0', border: '1px solid #ecc94b',
          fontSize: '0.8rem',
        }}>
          <span style={{ color: '#744210', fontWeight: '500' }}>
            ⚠ {allUnplaced.length} containers not placed
          </span>
          <Button
            size="small" variant="primary"
            onClick={() => onPlaceContainer(batch.id, allUnplaced)}
            style={{ fontSize: '11px', padding: '3px 10px', height: 'auto' }}
          >
            Place all ({allUnplaced.length})
          </Button>
        </div>
      )}

      {groups.map((g) => {
        // Формируем полный путь: Комната / Зона / Полка
        const fullLocation = [g.roomName, g.zoneName, g.positionName]
          .filter(part => part && String(part).trim().length > 0)
          .join(' / ');

        return (
          <div
            key={g.key}
            style={{
              display: 'flex', alignItems: 'center', gap: '8px',
              padding: '6px 10px', borderRadius: '8px',
              background: g.status === 'empty' ? '#fafafa' : g.isOpened ? '#fffaf0' : '#f7fafc',
              border: g.isOpened ? '1px solid #fbd38d' : '1px solid #e2e8f0',
              fontSize: '0.8rem',
            }}
          >
            {/* Count */}
            <span style={{
              fontWeight: '700', fontSize: '0.85rem', color: '#2d3748',
              minWidth: '32px', display: 'flex', alignItems: 'center', gap: '2px',
            }}>
              📦 {g.count}
            </span>

            {/* Total quantity */}
            <span style={{ fontWeight: '500', minWidth: '70px', color: '#1a365d' }}>
              {g.totalQty.toFixed(1)} {batch.unit}
            </span>

            {/* Status badge */}
            <GroupBadge status={g.status} isOpened={g.isOpened} />

            {/* Location & Actions */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginLeft: 'auto' }}>
              {g.positionId ? (
                <>
                  <span 
                    title={fullLocation} // Подсказка при наведении
                    style={{
                      fontSize: '11px', color: '#4a5568', background: '#edf2f7',
                      padding: '2px 8px', borderRadius: '4px',
                      display: 'inline-flex', alignItems: 'center', gap: '4px',
                      maxWidth: '300px', // Увеличили с 180px до 300px
                      overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    }}
                  >
                    <span style={{
                      width: 7, height: 7, borderRadius: '50%',
                      background: g.roomColor || '#a0aec0', flexShrink: 0,
                    }} />
                    {fullLocation}
                  </span>
                  {onMoveContainer && (
                    <Button
                      size="small" variant="ghost"
                      onClick={() => onMoveContainer(batch.id, g.containers)}
                      style={{ fontSize: '10px', padding: '2px 6px', height: 'auto' }}
                    >
                      Move {g.count > 1 ? `(${g.count})` : ''}
                    </Button>
                  )}
                </>
              ) : (
                <>
                  <span style={{ fontSize: '11px', color: '#cbd5e0', fontStyle: 'italic' }}>
                    not placed
                  </span>
                  {onPlaceContainer && (
                    <Button
                      size="small" variant="secondary"
                      onClick={() => onPlaceContainer(batch.id, g.containers)}
                      style={{ fontSize: '10px', padding: '2px 6px', height: 'auto' }}
                    >
                      Place {g.count > 1 ? `(${g.count})` : ''}
                    </Button>
                  )}
                </>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
};
// ==================== USAGE BAR ====================

const UsageBar = ({
  batch,
  containers,
  getUsageInput,
  setUsageQuantity,
  handleQuantityUse,
  adjustQuantityByPack,
  usageLoading,
  usageSuccess,
  usageError,
  usageContainer,
  setUsageContainer,
}) => {
  const available = batch.quantity - (batch.reserved_quantity || 0);
  const hasPackSize = batch.pack_size && batch.pack_size > 0;
  const input = getUsageInput(batch.id);
  const isLoading = usageLoading[batch.id];
  const success = usageSuccess[batch.id];
  const error = usageError[batch.id];

  const usableContainers = (containers || []).filter(
    c => getStatus(c) !== 'empty' && getStatus(c) !== 'disposed'
  );
  const hasMultipleContainers = usableContainers.length > 1;

  const selectedId = usageContainer[batch.id];
  const selectedContainer = usableContainers.find(c => c.id === selectedId);
  const maxQty = selectedContainer
    ? extractQty(selectedContainer)
    : available;

  const getFullLocation = (c) => {
    if (!c.room_name) return 'Not placed';
    return [c.room_name, c.zone_name, c.position_name]
      .filter(part => part && String(part).trim().length > 0)
      .join(' / ');
  };

  if (batch.status !== 'available' || available <= 0) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flex: 1 }}>
        <span style={{ fontSize: '0.8rem', color: '#a0aec0' }}>
          {batch.status === 'depleted' ? 'Depleted' : 'Not available for use'}
        </span>
      </div>
    );
  }

  const groupedOptions = usableContainers.reduce((acc, c) => {
    const loc = getFullLocation(c);
    if (!acc[loc]) acc[loc] = { opened: [], sealed: {} };

    const isOpened = 
      c.is_opened === true || c.is_opened === 1 || 
      String(c.is_opened).toLowerCase() === 'true' || 
      c.container_status === 'partial';

    if (isOpened) {
      acc[loc].opened.push(c);
    } else {
      const qty = extractQty(c);
      const key = String(qty);
      if (!acc[loc].sealed[key]) {
        acc[loc].sealed[key] = { 
          count: 0, firstId: c.id, firstSeq: c.sequence_number, qty: qty, unit: batch.unit 
        };
      }
      acc[loc].sealed[key].count += 1;
    }
    return acc;
  }, {});

  return (
    // КЛЮЧЕВОЕ ИЗМЕНЕНИЕ: Убрали width: 100%, добавили flex: 1
    // Теперь UsageBar делит строку с кнопками Action, прижимая их вправо
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', flex: 1, minWidth: '300px' }}>
      
      {/* Строка с элементами управления */}
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'flex-start', // Прижали всё влево
        gap: '12px', flexWrap: 'wrap'
      }}>
        
        {/* Выбор контейнера */}
        <div style={{ width: '320px', maxWidth: '100%', flexShrink: 1 }}>
          {hasMultipleContainers && (
            <select
              value={selectedId || ''}
              onChange={(e) => setUsageContainer(prev => ({ ...prev, [batch.id]: e.target.value || null }))}
              style={{
                height: '30px', fontSize: '12px', padding: '0 6px',
                border: '1px solid #e2e8f0', borderRadius: '6px',
                background: 'white', color: '#2d3748', textOverflow: 'ellipsis',
                width: '100%',
              }}
            >
              <option value="">Select container...</option>
              {Object.entries(groupedOptions).map(([location, data]) => (
                <optgroup key={location} label={location} style={{ fontStyle: 'normal', fontWeight: '700', color: '#1a202c' }}>
                  {data.opened.map(c => (
                    <option key={c.id} value={c.id} style={{ fontWeight: '400', paddingLeft: '10px' }}>
                      #{c.sequence_number} — {extractQty(c).toFixed(1)} {batch.unit} (opened)
                    </option>
                  ))}
                  {Object.values(data.sealed).map(g => (
                    <option key={g.firstId} value={g.firstId} style={{ fontWeight: '400', paddingLeft: '10px' }}>
                      {g.count > 1 
                        ? `${g.count} × ${g.qty.toFixed(1)} ${g.unit} (sealed)`
                        : `#${g.firstSeq} — ${g.qty.toFixed(1)} ${g.unit} (sealed)`}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          )}

          {usableContainers.length === 1 && (
            <span style={{
              fontSize: '11px', color: '#4a5568', background: '#edf2f7',
              padding: '4px 8px', borderRadius: '6px', whiteSpace: 'nowrap',
              display: 'inline-flex', alignItems: 'center', gap: '5px',
            }}>
              #{usableContainers[0].sequence_number}
              {usableContainers[0].room_name && (
                <>
                  <span style={{
                    width: 7, height: 7, borderRadius: '50%',
                    background: usableContainers[0].room_color || '#a0aec0',
                  }} />
                  <span style={{ fontWeight: '500', color: '#2d3748' }}>
                    {getFullLocation(usableContainers[0])}
                  </span>
                </>
              )}
              <span style={{ color: '#718096' }}>
                ({extractQty(usableContainers[0]).toFixed(1)} {batch.unit})
              </span>
            </span>
          )}
        </div>

        {/* Группа: Степпер + Ввод + Кнопка */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexShrink: 0 }}>
          
          {hasPackSize && (
            <div style={{
              display: 'flex', alignItems: 'center', border: '1px solid #e2e8f0',
              borderRadius: '6px', overflow: 'hidden', height: '30px', background: 'white',
            }}>
              <button
                type="button"
                onClick={() => adjustQuantityByPack(batch.id, batch.pack_size, 'down', maxQty)}
                disabled={isLoading || (parseFloat(input.quantity) || 0) <= 0}
                style={{
                  width: '26px', height: '100%', border: 'none', background: '#edf2f7',
                  cursor: 'pointer', fontSize: '14px', color: '#4a5568',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  borderRight: '1px solid #e2e8f0',
                }}
              >−</button>
              <span style={{ padding: '0 8px', fontSize: '12px', color: '#2d3748', fontWeight: '600', whiteSpace: 'nowrap' }}>
                {Math.floor((parseFloat(input.quantity) || 0) / batch.pack_size)} pcs
              </span>
              <button
                type="button"
                onClick={() => adjustQuantityByPack(batch.id, batch.pack_size, 'up', maxQty)}
                disabled={isLoading || (parseFloat(input.quantity) || 0) >= maxQty}
                style={{
                  width: '26px', height: '100%', border: 'none', background: '#edf2f7',
                  cursor: 'pointer', fontSize: '14px', color: '#4a5568',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  borderLeft: '1px solid #e2e8f0',
                }}
              >+</button>
            </div>
          )}

          <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
            <div style={{ display: 'flex', alignItems: 'center', position: 'relative' }}>
              <input
                type="number"
                step={hasPackSize ? batch.pack_size : '0.01'}
                min="0.01"
                max={maxQty}
                value={input.quantity}
                onChange={(e) => setUsageQuantity(batch.id, e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') { e.preventDefault(); handleQuantityUse(batch); }
                }}
                placeholder={`${Number(maxQty).toFixed(1)}`}
                disabled={isLoading}
                style={{
                  width: '95px', height: '30px', padding: '0 24px 0 8px',
                  border: error ? '1px solid #e53e3e' : '1px solid #e2e8f0',
                  borderRadius: '6px', fontSize: '13px', textAlign: 'right', fontWeight: '500',
                }}
              />
              <span style={{
                fontSize: '11px', color: '#718096', position: 'absolute', right: '6px',
                fontWeight: '600', pointerEvents: 'none'
              }}>
                {batch.unit}
              </span>
            </div>

            <Button
              size="small"
              variant="primary"
              onClick={() => handleQuantityUse(batch)}
              disabled={isLoading || !input.quantity || (hasMultipleContainers && !selectedId)}
              icon={<UseIcon size={14} />}
              style={{ backgroundColor: '#38a169', height: '30px', padding: '0 12px' }}
            >
              {isLoading ? '...' : 'Use'}
            </Button>
          </div>
        </div>
      </div>

      {/* Уведомления */}
      {(success || error) && (
        <div style={{ paddingLeft: '4px' }}>
          {success && <span style={{ color: '#38a169', fontSize: '12px', fontWeight: '600' }}>✓ {success}</span>}
          {error && <span style={{ color: '#e53e3e', fontSize: '12px' }} title={error}>⚠ {error}</span>}
        </div>
      )}
    </div>
  );
};

// ==================== BATCH EXPANDED CARD ====================

const BatchExpandedCard = ({
  batch, reagent, rooms, canEdit, canDeleteBatch,
  containers, containersLoading, loadContainers, splitBatch,
  onPlaceContainer, onMoveContainer,
  getUsageInput, setUsageQuantity, handleQuantityUse, adjustQuantityByPack,
  usageLoading, usageSuccess, usageError, usageContainer, setUsageContainer,
  onEdit, onDelete, onHistory, onRefresh,
}) => {
  const available = batch.quantity - (batch.reserved_quantity || 0);
  const hasPackSize = batch.pack_size && batch.pack_size > 0;

  useEffect(() => {
    if (loadContainers) loadContainers(batch.id);
  }, [batch.id, loadContainers]);

  const batchContainers = containers || [];
  const containerCount = batch.container_count || batchContainers.length;
  const openedCount = batch.opened_count || batchContainers.filter(c => c.is_opened).length;

  return (
    <div style={{
      margin: '0 0 8px 0', padding: '16px', backgroundColor: '#fff',
      borderRadius: '0 0 10px 10px', border: '1px solid #d0d8e4',
      borderTop: '2px solid #3182ce', animation: 'slideDown 0.15s ease-out',
    }}>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px', marginBottom: '16px' }}>
        <div>
          <div style={{ fontSize: '0.75rem', fontWeight: '700', color: '#718096', textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '8px' }}>
            Details
          </div>
          <DetailRow label="Supplier" value={batch.supplier} />
          <DetailRow label="Manufacturer" value={batch.manufacturer || reagent.manufacturer} />
          <DetailRow label="Catalog #" value={batch.cat_number} />
          <DetailRow label="Pack Size" value={hasPackSize ? `${batch.pack_size} ${batch.unit}` : null} />
          <DetailRow label="Containers" value={containerCount > 0 ? `${containerCount} (${openedCount} opened)` : null} />
          <DetailRow label="Reserved" value={(batch.reserved_quantity || 0) > 0 ? `${batch.reserved_quantity} ${batch.unit}` : null} color={(batch.reserved_quantity || 0) > 0 ? '#dd6b20' : undefined} />
          <DetailRow label="Available" value={`${available} ${batch.unit}`} color={available > 0 ? '#38a169' : '#e53e3e'} />
          <DetailRow label="Received" value={batch.received_date ? new Date(batch.received_date).toLocaleDateString() : null} />
          {batch.notes && <DetailRow label="Notes" value={batch.notes} />}
        </div>

        <div>
          <div style={{ fontSize: '0.75rem', fontWeight: '700', color: '#718096', textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '8px' }}>
            Containers
          </div>
          <GroupedContainerList
            batch={batch} containers={batchContainers} loading={containersLoading}
            onSplit={splitBatch} onRefresh={onRefresh}
            onPlaceContainer={onPlaceContainer} onMoveContainer={onMoveContainer}
          />
        </div>
      </div>

      {/* КЛЮЧЕВОЕ ИЗМЕНЕНИЕ: alignItems: 'flex-start' чтобы элементы не растягивались по вертикали */}
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start',
        paddingTop: '12px', borderTop: '1px solid #e2e8f0', flexWrap: 'wrap', gap: '16px',
      }}>
        <UsageBar
          batch={batch} containers={batchContainers}
          getUsageInput={getUsageInput} setUsageQuantity={setUsageQuantity}
          handleQuantityUse={handleQuantityUse} adjustQuantityByPack={adjustQuantityByPack}
          usageLoading={usageLoading} usageSuccess={usageSuccess} usageError={usageError}
          usageContainer={usageContainer} setUsageContainer={setUsageContainer}
        />
        <div style={{ display: 'flex', gap: '6px', flexShrink: 0 }}>
          <Button size="small" variant="ghost" onClick={() => onHistory(batch)} icon={<ClockIcon size={14} />}>History</Button>
          {canEdit && <Button size="small" variant="secondary" onClick={() => onEdit(batch)} icon={<EditIcon size={14} />}>Edit</Button>}
          {canDeleteBatch && <Button size="small" variant="danger" onClick={() => onDelete(batch)} icon={<TrashIcon size={14} />}>Delete</Button>}
        </div>
      </div>
    </div>
  );
};

export default BatchExpandedCard;