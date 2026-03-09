// components/Reagents/ReagentAccordionItem.js
// Compact batch rows → click → BatchExpandedCard with containers
// v2: использует PlacementModal для размещения контейнеров

import React, { useState } from 'react';
import Loading from '../Loading';
import Badge from '../Badge';
import Button from '../Button';
import { useRooms } from '../hooks/useRooms';
import { accordionStyles } from './styles';
import { PrinterIcon } from './icons';
import useBatchLogic from './useBatchLogic';
import PlacementModal from './PlacementModal';
import BatchExpandedCard from './BatchExpandedCard';

import {
  CreateBatchModal,
  EditBatchModal,
  UsageHistoryModal,
  PrintStickerModal
} from '../Modals';

import {
  ChevronRightIcon,
  EyeIcon,
  EditIcon,
  TrashIcon,
  PlusIcon,
  FlaskIcon,
  ClockIcon,
  DatabaseIcon
} from '../Icons';

// ==================== HELPERS ====================

const getExpiryStatus = (expiryDate) => {
  if (!expiryDate) return { style: {}, text: 'N/A' };
  const expiry = new Date(expiryDate);
  const now = new Date();
  const daysUntilExpiry = Math.ceil((expiry - now) / (1000 * 60 * 60 * 24));

  if (daysUntilExpiry < 0) {
    return { style: accordionStyles.expiryDanger, text: `Expired ${Math.abs(daysUntilExpiry)}d ago` };
  } else if (daysUntilExpiry <= 30) {
    return { style: accordionStyles.expiryWarning, text: `${daysUntilExpiry}d left` };
  } else {
    return { style: accordionStyles.expiryOk, text: expiry.toLocaleDateString() };
  }
};

const getStatusBadge = (status) => {
  const variants = {
    'available': 'success',
    'reserved': 'warning',
    'depleted': 'danger',
    'expired': 'danger',
    'low_stock': 'warning',
  };
  return <Badge variant={variants[status] || 'default'}>{status}</Badge>;
};

// Безопасное извлечение значения (на случай, если бэкенд отдает объект { parsedValue: 123 })
const getSafeValue = (val) => {
  if (typeof val === 'object' && val !== null) {
    return val.parsedValue !== undefined ? val.parsedValue : '';
  }
  return val;
};

// ==================== COMPACT CONTAINERS INDICATOR ====================

const CompactContainers = ({ batch }) => {
  const count = batch.container_count || batch.pack_count || 0;
  const opened = batch.opened_count || 0;
  const placed = batch.placed_count || 0;

  if (count === 0) {
    return <span style={{ fontSize: '11px', color: '#cbd5e0' }}>—</span>;
  }

  return (
    <span style={{
      fontSize: '11px', color: '#4a5568', display: 'inline-flex',
      alignItems: 'center', gap: '4px',
    }}>
      <span style={{ fontWeight: '600' }}>{count}</span>
      <span style={{ color: '#a0aec0' }}>
        ({opened > 0 ? `${opened}⊙` : 'sealed'})
      </span>
      {placed < count && (
        <span style={{ color: '#e53e3e', fontSize: '10px' }}>
          {count - placed} unplaced
        </span>
      )}
    </span>
  );
};

// ==================== COMPACT BATCH ROW ====================

const CompactBatchRow = ({ batch, isActive, onClick }) => {
  const [isHovered, setIsHovered] = useState(false);
  const expiryStatus = getExpiryStatus(batch.expiry_date);
  
  // Защита от объекта quantity
  const displayQuantity = getSafeValue(batch.quantity);

  return (
    <div
      style={{
        ...accordionStyles.batchRow,
        ...(isActive ? accordionStyles.batchRowActive : {}),
        ...(isHovered && !isActive ? accordionStyles.batchRowHover : {}),
      }}
      onClick={onClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Batch # */}
      <div style={{ ...accordionStyles.batchValue, fontWeight: '600' }}>
        {batch.batch_number}
      </div>

      {/* Quantity */}
      <div style={accordionStyles.batchValue}>
        {displayQuantity} {batch.unit}
      </div>

      {/* Status */}
      <div>{getStatusBadge(batch.status)}</div>

      {/* Containers (replaced Location) */}
      <div><CompactContainers batch={batch} /></div>

      {/* Expiry */}
      <div style={{ ...expiryStatus.style, fontSize: '0.8rem' }}>{expiryStatus.text}</div>

      {/* Chevron */}
      <div style={{
        ...accordionStyles.batchChevron,
        transform: isActive ? 'rotate(180deg)' : 'rotate(0deg)',
      }}>
        ▼
      </div>
    </div>
  );
};

// ==================== MAIN COMPONENT ====================

const ReagentAccordionItem = ({
  reagent,
  isExpanded,
  onToggle,
  onAction,
  onReagentsRefresh,
  canEdit,
  canDelete,
  canDeleteBatch,
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const [expandedBatchId, setExpandedBatchId] = useState(null);
  const { rooms } = useRooms();

  const logic = useBatchLogic(reagent.id, isExpanded, onReagentsRefresh);

  const toggleBatch = (batchId) => {
    setExpandedBatchId(prev => prev === batchId ? null : batchId);
  };

  const handlePrintClick = async (e) => {
    e.stopPropagation();
    if (logic.batches.length === 0) {
      await logic.loadBatches();
    }
    logic.setShowPrintModal(true);
  };

  return (
    <div style={accordionStyles.container}>
      {/* ===== Reagent Header ===== */}
      <div
        style={{ ...accordionStyles.header, ...(isHovered ? accordionStyles.headerHover : {}) }}
        onClick={onToggle}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        <div style={{
          ...accordionStyles.expandIcon,
          transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)'
        }}>
          <ChevronRightIcon size={16} color="#3182ce" />
        </div>

        <div style={accordionStyles.reagentInfo}>
          <div style={accordionStyles.reagentName} title={reagent.name}>{reagent.name}</div>
          <div style={accordionStyles.reagentField} title={reagent.formula}>{reagent.formula || '—'}</div>
          <div style={accordionStyles.reagentField}>{reagent.molecular_weight || '—'}</div>
          <div style={accordionStyles.reagentField} title={reagent.cas_number}>{reagent.cas_number || '—'}</div>
          <div>
            <Badge variant={reagent.status === 'active' ? 'success' : 'warning'}>
              {reagent.status || 'Unknown'}
            </Badge>
          </div>
          <div style={{
            ...accordionStyles.reagentField,
            color: reagent.total_display === 'No stock' ? '#e53e3e' : '#38a169',
            fontWeight: '600'
          }}>
            {reagent.total_display || `${reagent.total_quantity} ${reagent.primary_unit || ''}`}
          </div>
        </div>

        <div style={accordionStyles.actionsColumn} onClick={e => e.stopPropagation()}>
          <Button size="small" variant="ghost" onClick={() => onAction('view', reagent)} icon={<EyeIcon size={14} />}>View</Button>
          <Button size="small" variant="secondary" onClick={handlePrintClick} icon={<PrinterIcon size={14} />} title="Print Stickers">Print</Button>
          {canEdit && <Button size="small" variant="primary" onClick={() => onAction('edit', reagent)} icon={<EditIcon size={14} />}>Edit</Button>}
          {canDelete && <Button size="small" variant="danger" onClick={() => onAction('delete', reagent)} icon={<TrashIcon size={14} />}>Delete</Button>}
        </div>
      </div>

      {/* ===== Batches Section ===== */}
      {isExpanded && (
        <div style={accordionStyles.batchesContainer}>
          <div style={accordionStyles.batchesHeader}>
            <span style={accordionStyles.batchesTitle}>
              <DatabaseIcon size={16} color="#3182ce" /> Batches ({logic.batches.length})
            </span>
            <div style={{ display: 'flex', gap: '8px' }}>
              {logic.batches.length > 0 && (
                <Button size="small" variant="secondary" onClick={() => logic.setShowPrintModal(true)} icon={<PrinterIcon size={14} />}>Print All</Button>
              )}
              <Button size="small" variant="primary" onClick={() => logic.setShowCreateBatch(true)} icon={<PlusIcon size={14} />}>Add Batch</Button>
            </div>
          </div>

          {logic.loadingBatches ? (
            <div style={{ textAlign: 'center', padding: '20px' }}><Loading /></div>
          ) : logic.batches.length === 0 ? (
            <div style={accordionStyles.noBatches}>
              <FlaskIcon size={24} color="#a0aec0" style={{ marginBottom: '8px' }} />
              <p style={{ margin: 0 }}>No batches found.</p>
            </div>
          ) : (
            <div>
              {/* Column headers */}
              <div style={accordionStyles.batchRowHeader}>
                <div>Batch #</div>
                <div>Qty</div>
                <div>Status</div>
                <div>Containers</div>
                <div>Expiry</div>
                <div></div>
              </div>

              {/* Batch rows */}
              {logic.batches.map(batch => (
                <React.Fragment key={batch.id}>
                  <CompactBatchRow
                    batch={batch}
                    isActive={expandedBatchId === batch.id}
                    onClick={() => toggleBatch(batch.id)}
                  />

                  {/* Expanded card */}
                  {expandedBatchId === batch.id && (
                    <BatchExpandedCard
                      batch={batch}
                      reagent={reagent}
                      rooms={rooms}
                      canEdit={canEdit}
                      canDeleteBatch={canDeleteBatch}
                      
                      // Container data
                      containers={logic.containersMap[batch.id] || []}
                      containersLoading={logic.containersLoading[batch.id]}
                      loadContainers={logic.loadContainers}
                      splitBatch={logic.splitBatch}
                      onPlaceContainer={logic.placeContainer} 
                      onMoveContainer={logic.moveContainer}   

                      // Usage logic
                      getUsageInput={logic.getUsageInput}
                      setUsageQuantity={logic.setUsageQuantity}
                      handleQuantityUse={logic.handleQuantityUse}
                      adjustQuantityByPack={logic.adjustQuantityByPack}
                      usageLoading={logic.usageLoading}
                      usageSuccess={logic.usageSuccess}
                      usageError={logic.usageError}
                      usageContainer={logic.usageContainer}
                      setUsageContainer={logic.setUsageContainer}
                      
                      // Actions
                      onEdit={(b) => { logic.setSelectedBatch(b); logic.setShowEditBatch(true); }}
                      onDelete={logic.handleDeleteBatch}
                      onHistory={(b) => { logic.setSelectedBatch(b); logic.setShowUsageHistory(true); }}
                      onRefresh={logic.loadBatches}
                    />
                  )}
                </React.Fragment>
              ))}
            </div>
          )}

          {/* Modals */}
          {logic.showCreateBatch && (
            <CreateBatchModal
              isOpen={logic.showCreateBatch}
              reagentId={reagent.id}
              reagentName={reagent.name}
              onClose={() => logic.setShowCreateBatch(false)}
              onSave={logic.handleBatchCreated}
            />
          )}
          {logic.showEditBatch && logic.selectedBatch && (
            <EditBatchModal
              isOpen={logic.showEditBatch}
              batch={logic.selectedBatch}
              onClose={() => { logic.setShowEditBatch(false); logic.setSelectedBatch(null); }}
              onSave={logic.handleBatchUpdated}
            />
          )}
          {logic.showUsageHistory && logic.selectedBatch && (
            <UsageHistoryModal
              isOpen={logic.showUsageHistory}
              reagentId={reagent.id}
              batchId={logic.selectedBatch.id}
              batch={logic.selectedBatch}
              onClose={() => { logic.setShowUsageHistory(false); logic.setSelectedBatch(null); }}
              onSave={logic.handleBatchUpdated}
            />
          )}
        </div>
      )}

      {/* Print Modal */}
      {logic.showPrintModal && (
        <PrintStickerModal
          isOpen={logic.showPrintModal}
          onClose={() => { logic.setShowPrintModal(false); logic.setSelectedBatch(null); }}
          reagent={reagent}
          batches={logic.batches}
          preSelectedBatchId={logic.selectedBatch?.id}
        />
      )}
      
      {/* ===== Placement Modal (v2 — со stepper) ===== */}
      {logic.containerToPlace && (
        <PlacementModal
          isOpen={!!logic.containerToPlace}
          containers={logic.containerToPlace.containers}
          action={logic.containerToPlace.action}
          batchId={logic.containerToPlace.batchId}
          onConfirm={async (assignments) => {
            await logic.handleLocationSave(assignments);
          }}
          onClose={() => logic.setContainerToPlace(null)}
        />
      )}
    </div>
  );
};

export default ReagentAccordionItem;
