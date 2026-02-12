// src/components/Reagents.js
import React, { useState, useEffect, useMemo } from 'react';
import useReagents from './hooks/useReagents';
import { api } from '../services/api';
import ErrorMessage from './ErrorMessage';
import Loading from './Loading';
import Badge from './Badge';
import Button from './Button';
import Input from './Input';
import Select from './Select';

import {
  CreateReagentModal,
  EditReagentModal,
  ViewReagentModal,
  CreateBatchModal,
  EditBatchModal,
  UsageHistoryModal,
  PrintStickerModal
} from './Modals';
import BatchImportModal from './BatchImportModal';

import {
  SearchIcon,
  FilterIcon,
  PlusIcon,
  UploadIcon,
  ChevronRightIcon,
  ChevronLeftIcon,
  EyeIcon,
  EditIcon,
  TrashIcon,
  CloseIcon,
  FlaskIcon,
  ClockIcon,
  DatabaseIcon
} from './Icons';

// Use icon for consuming reagents
const UseIcon = ({ size = 24, color = "currentColor" }) => (
    <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5 12h14"></path>
      <circle cx="12" cy="12" r="10"></circle>
    </svg>
);

// Local PrinterIcon fallback
const PrinterIcon = ({ size = 24, color = "currentColor" }) => (
    <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="6 9 6 2 18 2 18 9"></polyline>
      <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path>
      <rect x="6" y="14" width="12" height="8"></rect>
    </svg>
);

const COLUMN_WIDTHS = {
  expandIcon: '32px',
  actions: '260px',
  gridColumns: '2fr 1fr 80px 1fr 1fr 100px'
};

const accordionStyles = {
  container: {
    border: '1px solid #e2e8f0',
    borderRadius: '12px',
    overflow: 'hidden',
    marginBottom: '8px',
    backgroundColor: '#fff',
    boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)',
    transition: 'all 0.2s ease'
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '14px 18px',
    backgroundColor: '#fff',
    cursor: 'pointer',
    userSelect: 'none',
    transition: 'all 0.2s ease',
    gap: '14px'
  },
  headerHover: {
    backgroundColor: 'rgba(49, 130, 206, 0.04)'
  },
  expandIcon: {
    width: '28px',
    height: '28px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: '#3182ce',
    transition: 'transform 0.2s ease',
    flexShrink: 0,
    backgroundColor: 'rgba(49, 130, 206, 0.1)',
    borderRadius: '8px'
  },
  reagentInfo: {
    flex: 1,
    display: 'grid',
    gridTemplateColumns: COLUMN_WIDTHS.gridColumns,
    gap: '12px',
    alignItems: 'center'
  },
  reagentName: {
    fontWeight: '600',
    color: '#1a365d',
    fontSize: '0.95rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap'
  },
  reagentField: {
    color: '#718096',
    fontSize: '0.875rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap'
  },
  actionsColumn: {
    width: COLUMN_WIDTHS.actions,
    display: 'flex',
    gap: '6px',
    flexShrink: 0,
    justifyContent: 'flex-start'
  },
  batchesContainer: {
    borderTop: '1px solid #e2e8f0',
    backgroundColor: '#f8fafc',
    padding: '18px',
    animation: 'slideDown 0.2s ease-out'
  },
  batchesHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '14px'
  },
  batchesTitle: {
    fontSize: '0.875rem',
    fontWeight: '700',
    color: '#1a365d',
    display: 'flex',
    alignItems: 'center',
    gap: '8px'
  },
  batchCard: {
    display: 'grid',
    gridTemplateColumns: '100px 80px 70px 70px 100px 90px 90px 70px 80px 90px 90px 1fr',
    gap: '12px',
    padding: '14px',
    backgroundColor: '#fff',
    borderRadius: '10px',
    border: '1px solid #e2e8f0',
    marginBottom: '8px',
    alignItems: 'center',
    transition: 'all 0.2s ease'
  },
  batchValue: {
    color: '#1a365d',
    fontWeight: '500',
    fontSize: '0.875rem'
  },
  noBatches: {
    textAlign: 'center',
    color: '#a0aec0',
    padding: '24px',
    fontSize: '0.875rem',
    backgroundColor: '#fff',
    borderRadius: '10px',
    border: '1px dashed #e2e8f0'
  },
  expiryWarning: { color: '#ed8936', fontWeight: '600' },
  expiryDanger: { color: '#e53e3e', fontWeight: '600' },
  expiryOk: { color: '#38a169' }
};

const ReagentAccordionItem = ({
                                reagent,
                                isExpanded,
                                onToggle,
                                onAction,
                                onReagentsRefresh,
                                canEdit,
                                canDelete
                              }) => {
  const [batches, setBatches] = useState([]);
  const [loadingBatches, setLoadingBatches] = useState(false);
  const [isHovered, setIsHovered] = useState(false);

  const [showCreateBatch, setShowCreateBatch] = useState(false);
  const [showEditBatch, setShowEditBatch] = useState(false);
  const [showUsageHistory, setShowUsageHistory] = useState(false);
  const [showPrintModal, setShowPrintModal] = useState(false);
  const [selectedBatch, setSelectedBatch] = useState(null);

  // Инлайн-списание: состояние для каждого батча
  const [usageInputs, setUsageInputs] = useState({}); // { batchId: { quantity: '' } }
  const [usageLoading, setUsageLoading] = useState({}); // { batchId: true/false }
  const [usageSuccess, setUsageSuccess] = useState({}); // { batchId: 'message' }
  const [usageError, setUsageError] = useState({}); // { batchId: 'error' }

  useEffect(() => {
    if (isExpanded && batches.length === 0) {
      loadBatches();
    }
    // eslint-disable-next-line
  }, [isExpanded]);

  const loadBatches = async () => {
    setLoadingBatches(true);
    try {
      const data = await api.getReagentBatches(reagent.id);
      setBatches(Array.isArray(data) ? data : (data.data || []));
    } catch (err) {
      console.error('Failed to load batches:', err);
    } finally {
      setLoadingBatches(false);
    }
  };

  const handleBatchCreated = () => {
    setShowCreateBatch(false);
    loadBatches();
    if (onReagentsRefresh) onReagentsRefresh();
  };

  const handleBatchUpdated = () => {
    setShowEditBatch(false);
    setSelectedBatch(null);
    loadBatches();
    if (onReagentsRefresh) onReagentsRefresh();
  };

  const handleDeleteBatch = async (batch) => {
    if (window.confirm(`Delete batch "${batch.batch_number}"?`)) {
      try {
        await api.deleteBatch(batch.reagent_id, batch.id);
        loadBatches();
        if (onReagentsRefresh) onReagentsRefresh();
      } catch (err) {
        alert(err.message || 'Failed to delete batch');
      }
    }
  };

  // ========== Инлайн-списание ==========
  
  const getUsageInput = (batchId) => usageInputs[batchId] || { quantity: '' };
  
  const setUsageQuantity = (batchId, value) => {
    setUsageInputs(prev => ({
      ...prev,
      [batchId]: { quantity: value }
    }));
    setUsageError(prev => ({ ...prev, [batchId]: '' }));
  };

  // Обычное списание по количеству
  const handleQuantityUse = async (batch) => {
    const input = getUsageInput(batch.id);
    const qty = parseFloat(input.quantity);
    
    if (!qty || qty <= 0) {
      setUsageError(prev => ({ ...prev, [batch.id]: 'Enter quantity' }));
      return;
    }
    
    const available = batch.quantity - (batch.reserved_quantity || 0);
    if (qty > available) {
      setUsageError(prev => ({ ...prev, [batch.id]: `Max: ${available}` }));
      return;
    }

    setUsageLoading(prev => ({ ...prev, [batch.id]: true }));
    setUsageError(prev => ({ ...prev, [batch.id]: '' }));
    
    try {
      await api.useReagent(reagent.id, batch.id, { quantity_used: qty });
      setUsageSuccess(prev => ({ ...prev, [batch.id]: `−${qty} ${batch.unit}` }));
      setUsageQuantity(batch.id, '');
      loadBatches();
      if (onReagentsRefresh) onReagentsRefresh();
      setTimeout(() => setUsageSuccess(prev => ({ ...prev, [batch.id]: '' })), 2500);
    } catch (err) {
      setUsageError(prev => ({ ...prev, [batch.id]: err.message || 'Error' }));
    } finally {
      setUsageLoading(prev => ({ ...prev, [batch.id]: false }));
    }
  };

  // Штучное списание (по pack_size)
  const handlePackUse = async (batch, packCount = 1) => {
    if (!batch.pack_size || batch.pack_size <= 0) {
      setUsageError(prev => ({ ...prev, [batch.id]: 'No pack size' }));
      return;
    }
    
    const qty = batch.pack_size * packCount;
    const available = batch.quantity - (batch.reserved_quantity || 0);
    
    if (qty > available) {
      setUsageError(prev => ({ ...prev, [batch.id]: `Max: ${Math.floor(available / batch.pack_size)} pcs` }));
      return;
    }

    setUsageLoading(prev => ({ ...prev, [batch.id]: true }));
    setUsageError(prev => ({ ...prev, [batch.id]: '' }));
    
    try {
      // Используем обычный use endpoint с вычисленным количеством
      await api.useReagent(reagent.id, batch.id, { quantity_used: qty });
      setUsageSuccess(prev => ({ ...prev, [batch.id]: `−${packCount} pcs` }));
      setUsageQuantity(batch.id, '');
      loadBatches();
      if (onReagentsRefresh) onReagentsRefresh();
      setTimeout(() => setUsageSuccess(prev => ({ ...prev, [batch.id]: '' })), 2500);
    } catch (err) {
      setUsageError(prev => ({ ...prev, [batch.id]: err.message || 'Error' }));
    } finally {
      setUsageLoading(prev => ({ ...prev, [batch.id]: false }));
    }
  };

  // Stepper для packs: увеличить/уменьшить на 1 pack_size
  const adjustQuantityByPack = (batchId, packSize, direction, available) => {
    const current = parseFloat(getUsageInput(batchId).quantity) || 0;
    let newValue;
    
    if (direction === 'up') {
      newValue = current + packSize;
      if (newValue > available) newValue = available;
    } else {
      newValue = current - packSize;
      if (newValue < 0) newValue = 0;
    }
    
    setUsageQuantity(batchId, newValue > 0 ? newValue.toString() : '');
  };

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
      'expired': 'danger'
    };
    return <Badge variant={variants[status] || 'default'}>{status}</Badge>;
  };

  const handlePrintClick = async (e) => {
    e.stopPropagation();
    if (batches.length === 0) {
      setLoadingBatches(true);
      try {
        const data = await api.getReagentBatches(reagent.id);
        const loadedBatches = Array.isArray(data) ? data : (data.data || []);
        setBatches(loadedBatches);
        if (loadedBatches.length > 0) {
          setShowPrintModal(true);
        } else {
          alert('No batches available for this reagent');
        }
      } catch (err) {
        alert('Failed to load batches');
      } finally {
        setLoadingBatches(false);
      }
    } else {
      setShowPrintModal(true);
    }
  };

  return (
      <div style={accordionStyles.container}>
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
            <Button
                size="small"
                variant="secondary"
                onClick={handlePrintClick}
                icon={<PrinterIcon size={14} />}
                title="Print Stickers"
            >
              Print
            </Button>
            {canEdit && <Button size="small" variant="primary" onClick={() => onAction('edit', reagent)} icon={<EditIcon size={14} />}>Edit</Button>}
            {canDelete && <Button size="small" variant="danger" onClick={() => onAction('delete', reagent)} icon={<TrashIcon size={14} />}>Delete</Button>}
          </div>
        </div>

        {isExpanded && (
            <div style={accordionStyles.batchesContainer}>
              <div style={accordionStyles.batchesHeader}>
            <span style={accordionStyles.batchesTitle}>
              <DatabaseIcon size={16} color="#3182ce" /> Batches ({batches.length})
            </span>
                <div style={{ display: 'flex', gap: '8px' }}>
                  {batches.length > 0 && (
                      <Button
                          size="small"
                          variant="secondary"
                          onClick={() => setShowPrintModal(true)}
                          icon={<PrinterIcon size={14} />}
                      >
                        Print All
                      </Button>
                  )}
                  <Button size="small" variant="primary" onClick={() => setShowCreateBatch(true)} icon={<PlusIcon size={14} />}>Add Batch</Button>
                </div>
              </div>

              {loadingBatches ? (
                  <div style={{ textAlign: 'center', padding: '20px' }}><Loading /></div>
              ) : batches.length === 0 ? (
                  <div style={accordionStyles.noBatches}>
                    <FlaskIcon size={24} color="#a0aec0" style={{ marginBottom: '8px' }} />
                    <p style={{ margin: 0 }}>No batches found.</p>
                  </div>
              ) : (
                  <div>
                    <div style={{
                      ...accordionStyles.batchCard,
                      background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.08) 0%, rgba(56, 161, 105, 0.08) 100%)',
                      fontWeight: '700', fontSize: '0.7rem', color: '#1a365d', textTransform: 'uppercase', letterSpacing: '0.05em', border: 'none'
                    }}>
                      <div>Batch #</div><div>Qty</div><div>Packs</div><div>Pack Size</div><div>Manufacturer</div><div>Catalog #</div><div>Supplier</div><div>Reserved</div><div>Status</div><div>Expiry</div><div>Location</div><div>Actions</div>
                    </div>

                    {batches.map(batch => {
                      const expiryStatus = getExpiryStatus(batch.expiry_date);
                      const input = getUsageInput(batch.id);
                      const isLoading = usageLoading[batch.id];
                      const success = usageSuccess[batch.id];
                      const error = usageError[batch.id];
                      const available = batch.quantity - (batch.reserved_quantity || 0);
                      const hasPackSize = batch.pack_size && batch.pack_size > 0;
                      const packCount = hasPackSize ? Math.floor(batch.quantity / batch.pack_size) : 0;
                      const availablePacks = hasPackSize ? Math.floor(available / batch.pack_size) : 0;
                      const inputQty = parseFloat(input.quantity) || 0;
                      const inputPacks = hasPackSize && inputQty > 0 ? Math.floor(inputQty / batch.pack_size) : 0;
                      
                      return (
                          <div key={batch.id} style={accordionStyles.batchCard}>
                            <div style={accordionStyles.batchValue}>{batch.batch_number}</div>
                            <div style={accordionStyles.batchValue}>{batch.quantity} {batch.unit}</div>
                            
                            {/* Packs column with vertical stepper */}
                            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '2px' }}>
                              {hasPackSize && availablePacks > 0 && batch.status === 'available' ? (
                                <>
                                  <button
                                    type="button"
                                    onClick={() => adjustQuantityByPack(batch.id, batch.pack_size, 'up', available)}
                                    disabled={isLoading || inputQty >= available}
                                    style={{
                                      width: '32px',
                                      height: '18px',
                                      border: '1px solid #cbd5e0',
                                      borderRadius: '4px',
                                      background: isLoading || inputQty >= available ? '#f7fafc' : '#edf2f7',
                                      cursor: isLoading || inputQty >= available ? 'not-allowed' : 'pointer',
                                      fontSize: '12px',
                                      color: isLoading || inputQty >= available ? '#a0aec0' : '#4a5568',
                                      padding: 0,
                                      display: 'flex',
                                      alignItems: 'center',
                                      justifyContent: 'center'
                                    }}
                                  >▲</button>
                                  <span style={{ 
                                    color: inputPacks > 0 ? '#3182ce' : '#718096', 
                                    fontWeight: inputPacks > 0 ? '600' : '500',
                                    fontSize: '0.85rem',
                                    whiteSpace: 'nowrap'
                                  }}>
                                    {inputPacks > 0 ? `${inputPacks}/` : ''}{packCount} pcs
                                  </span>
                                  <button
                                    type="button"
                                    onClick={() => adjustQuantityByPack(batch.id, batch.pack_size, 'down', available)}
                                    disabled={isLoading || inputQty <= 0}
                                    style={{
                                      width: '32px',
                                      height: '18px',
                                      border: '1px solid #cbd5e0',
                                      borderRadius: '4px',
                                      background: isLoading || inputQty <= 0 ? '#f7fafc' : '#edf2f7',
                                      cursor: isLoading || inputQty <= 0 ? 'not-allowed' : 'pointer',
                                      fontSize: '12px',
                                      color: isLoading || inputQty <= 0 ? '#a0aec0' : '#4a5568',
                                      padding: 0,
                                      display: 'flex',
                                      alignItems: 'center',
                                      justifyContent: 'center'
                                    }}
                                  >▼</button>
                                </>
                              ) : (
                                <span style={{ color: packCount ? '#3182ce' : '#a0aec0', fontSize: '0.85rem' }}>
                                  {packCount ? `${packCount} pcs` : '—'}
                                </span>
                              )}
                            </div>
                            
                            {/* Pack Size column */}
                            <div style={accordionStyles.batchValue} title={batch.pack_size ? `${batch.pack_size} ${batch.unit}` : ''}>
                              {batch.pack_size ? `${batch.pack_size} ${batch.unit}` : '—'}
                            </div>
                            
                            {/* Manufacturer column */}
                            <div style={{ ...accordionStyles.batchValue, fontSize: '0.8rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={batch.manufacturer || reagent.manufacturer}>
                              {batch.manufacturer || reagent.manufacturer || '—'}
                            </div>
                            
                            {/* Catalog Number column */}
                            <div style={{ ...accordionStyles.batchValue, fontSize: '0.8rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={batch.cat_number}>
                              {batch.cat_number || '—'}
                            </div>
                            
                            {/* Supplier column */}
                            <div style={{ ...accordionStyles.batchValue, fontSize: '0.8rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={batch.supplier}>
                              {batch.supplier || '—'}
                            </div>
                            
                            <div style={{ ...accordionStyles.batchValue, color: (batch.reserved_quantity||0) > 0 ? '#dd6b20' : '#a0aec0' }}>
                              {(batch.reserved_quantity||0) > 0 ? `${batch.reserved_quantity} ${batch.unit}` : '—'}
                            </div>
                            <div>{getStatusBadge(batch.status)}</div>
                            <div style={expiryStatus.style}>{expiryStatus.text}</div>
                            <div style={accordionStyles.batchValue}>{batch.storage_location || batch.location || '—'}</div>
                            
                            {/* Actions column */}
                            <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexWrap: 'nowrap' }}>
                              {/* Quantity input */}
                              {batch.status === 'available' && available > 0 && (
                                <>
                                  <input
                                    type="number"
                                    step={hasPackSize ? batch.pack_size : "0.01"}
                                    min="0.01"
                                    max={available}
                                    value={input.quantity}
                                    onChange={(e) => setUsageQuantity(batch.id, e.target.value)}
                                    onKeyDown={(e) => {
                                      if (e.key === 'Enter') {
                                        e.preventDefault();
                                        handleQuantityUse(batch);
                                      }
                                    }}
                                    placeholder={available.toString()}
                                    disabled={isLoading}
                                    style={{
                                      width: '70px',
                                      height: '30px',
                                      padding: '0 6px',
                                      border: error ? '1px solid #e53e3e' : '1px solid #e2e8f0',
                                      borderRadius: '6px',
                                      fontSize: '13px',
                                      textAlign: 'right'
                                    }}
                                  />
                                  <span style={{ fontSize: '11px', color: '#718096', minWidth: '20px' }}>
                                    {batch.unit}
                                  </span>
                                  <Button 
                                    size="small" 
                                    variant="primary" 
                                    onClick={() => handleQuantityUse(batch)} 
                                    disabled={isLoading || !input.quantity}
                                    icon={<UseIcon size={14} />} 
                                    style={{ backgroundColor: '#38a169' }}
                                  >
                                    {isLoading ? '...' : 'Use'}
                                  </Button>
                                </>
                              )}
                              
                              {/* Success indicator */}
                              {success && (
                                <span style={{ color: '#38a169', fontSize: '11px', fontWeight: '600', whiteSpace: 'nowrap' }}>
                                  ✓ {success}
                                </span>
                              )}
                              
                              {/* Error indicator */}
                              {error && (
                                <span style={{ color: '#e53e3e', fontSize: '11px' }} title={error}>⚠</span>
                              )}
                              
                              {/* History button */}
                              <Button size="small" variant="ghost" onClick={() => { setSelectedBatch(batch); setShowUsageHistory(true); }} icon={<ClockIcon size={14} />}>History</Button>
                              
                              {/* Edit & Delete */}
                              {canEdit && <Button size="small" variant="secondary" onClick={() => { setSelectedBatch(batch); setShowEditBatch(true); }} icon={<EditIcon size={14} />}>Edit</Button>}
                              {canDelete && <Button size="small" variant="danger" onClick={() => handleDeleteBatch(batch)} icon={<TrashIcon size={14} />}>Delete</Button>}
                            </div>
                          </div>
                      );
                    })}
                  </div>
              )}

              {showCreateBatch && <CreateBatchModal isOpen={showCreateBatch} reagentId={reagent.id} reagentName={reagent.name} onClose={() => setShowCreateBatch(false)} onSave={handleBatchCreated} />}
              {showEditBatch && selectedBatch && <EditBatchModal isOpen={showEditBatch} batch={selectedBatch} onClose={() => { setShowEditBatch(false); setSelectedBatch(null); }} onSave={handleBatchUpdated} />}
              {showUsageHistory && selectedBatch && <UsageHistoryModal isOpen={showUsageHistory} reagentId={reagent.id} batchId={selectedBatch.id} batch={selectedBatch} onClose={() => { setShowUsageHistory(false); setSelectedBatch(null); }} onSave={handleBatchUpdated} />}
            </div>
        )}

        {showPrintModal && (
            <PrintStickerModal
                isOpen={showPrintModal}
                onClose={() => { setShowPrintModal(false); setSelectedBatch(null); }}
                reagent={reagent}
                batches={batches}
                preSelectedBatchId={selectedBatch?.id}
            />
        )}
      </div>
  );
};

const Reagents = ({ user }) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [manufacturerFilter, setManufacturerFilter] = useState('');
  const [stockFilter, setStockFilter] = useState('');
  const [casNumberFilter, setCasNumberFilter] = useState('');
  const [sortBy, setSortBy] = useState('created_at');
  const [sortOrder, setSortOrder] = useState('desc');
  const [showFilters, setShowFilters] = useState(false);

  const activeFilters = useMemo(() => {
    const filters = {};
    if (searchTerm) filters.search = searchTerm;
    if (statusFilter) filters.status = statusFilter;
    if (manufacturerFilter) filters.manufacturer = manufacturerFilter;
    if (stockFilter) filters.stock_status = stockFilter;
    if (casNumberFilter) filters.cas_number = casNumberFilter;
    return filters;
  }, [searchTerm, statusFilter, manufacturerFilter, stockFilter, casNumberFilter]);

  // ИНИЦИАЛИЗАЦИЯ ХУКА (useCursor: false для страничной пагинации)
  const {
    data: reagents,
    loading,
    error,
    pagination,
    sorting,
    refresh,
    actions
  } = useReagents(activeFilters, {
    initialPerPage: 20,
    useCursor: false
  });

  useEffect(() => {
    // Используем setSortFull для установки обоих параметров сразу
    if (sorting.setSortFull) {
      sorting.setSortFull(sortBy, sortOrder);
    } else {
      // Fallback для старой версии хука
      sorting.setSort(sortBy);
    }
    // eslint-disable-next-line
  }, [sortBy, sortOrder]);

  const [expandedReagents, setExpandedReagents] = useState(new Set());

  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showViewModal, setShowViewModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [selectedReagent, setSelectedReagent] = useState(null);

  const [allBatches, setAllBatches] = useState([]);
  const [manufacturers, setManufacturers] = useState(['Sigma-Aldrich', 'Merck', 'Thermo Fisher', 'VWR', 'Alfa Aesar']);

  useEffect(() => {
    if (reagents.length > 0) {
      const uniqueManufacturers = [...new Set(reagents.map(r => r.manufacturer).filter(Boolean))].sort();
      setManufacturers(prev => [...new Set([...prev, ...uniqueManufacturers])].sort());
    }
  }, [reagents]);

  useEffect(() => {
    const loadReferenceData = async () => {
      try {
        const batchesData = await api.getAllBatches();
        setAllBatches(Array.isArray(batchesData) ? batchesData : (batchesData.data || []));
      } catch (e) {
        console.error("Failed to load reference data:", e);
      }
    };
    loadReferenceData();
  }, []);

  // Debug: uncomment to see user role
  // console.log('User role:', user?.role, 'typeof:', typeof user?.role);
  
  const userRole = (user?.role || '').toString().toLowerCase();
  const isAdmin = userRole === 'admin';
  const isResearcher = userRole === 'researcher';
  const canEditReagents = () => isAdmin || isResearcher;

  const handleAction = async (action, reagent) => {
    switch (action) {
      case 'view':
        setSelectedReagent(reagent);
        setShowViewModal(true);
        break;
      case 'edit':
        setSelectedReagent(reagent);
        setShowEditModal(true);
        break;
      case 'delete':
        if (window.confirm(`Delete reagent "${reagent.name}"? This will also delete all associated batches.`)) {
          try {
            await api.deleteReagent(reagent.id);
            actions.removeItem(reagent.id);
          } catch (err) {
            alert(err.message || 'Failed to delete reagent');
          }
        }
        break;
      default: break;
    }
  };

  const handleCreateSuccess = () => {
    setShowCreateModal(false);
    refresh();
  };

  const handleEditSuccess = () => {
    setShowEditModal(false);
    setSelectedReagent(null);
    refresh();
  };

  const handleImport = async (importData) => {
    try {
      if (importData instanceof File) {
        await api.importReagents(importData);
      } else {
        const blob = new Blob([JSON.stringify(importData)], { type: 'application/json' });
        const file = new File([blob], 'import.json', { type: 'application/json' });
        await api.importReagents(file);
      }
      setShowImportModal(false);
      refresh();
    } catch (err) {
      console.error('Failed to import:', err);
      throw err;
    }
  };

  const toggleAccordion = (reagentId) => {
    setExpandedReagents(prev => {
      const newSet = new Set(prev);
      if (newSet.has(reagentId)) newSet.delete(reagentId);
      else newSet.add(reagentId);
      return newSet;
    });
  };

  const clearFilters = () => {
    setSearchTerm('');
    setStatusFilter('');
    setManufacturerFilter('');
    setStockFilter('');
    setCasNumberFilter('');
    setSortBy('created_at');
    setSortOrder('desc');
  };

  const activeFiltersCount = [searchTerm, statusFilter, manufacturerFilter, stockFilter, casNumberFilter].filter(Boolean).length;

  if (error) return <ErrorMessage message={error} onRetry={refresh} />;

  return (
      <div style={{ padding: '20px', paddingTop: '90px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' }}>
          <div>
            <h1 style={{ margin: 0, fontSize: '1.75rem', fontWeight: '800', color: '#1a365d' }}>Reagents</h1>
            <p style={{ margin: '4px 0 0 0', color: '#718096', fontSize: '0.875rem' }}>Manage your laboratory reagents and batches</p>
          </div>
          <div style={{ display: 'flex', gap: '10px' }}>
            <Button variant="secondary" onClick={() => setShowImportModal(true)} icon={<UploadIcon size={16} />}>Import</Button>
            {canEditReagents() && (
                <Button variant="primary" onClick={() => setShowCreateModal(true)} icon={<PlusIcon size={16} />}>Add Reagent</Button>
            )}
          </div>
        </div>

        <div style={{ marginBottom: '20px' }}>
          <div style={{ display: 'flex', gap: '10px', marginBottom: '10px' }}>
            <div style={{ flex: 1, position: 'relative' }}>
              <div style={{ position: 'absolute', left: '14px', top: '50%', transform: 'translateY(-50%)', pointerEvents: 'none' }}>
                <SearchIcon size={18} color="#a0aec0" />
              </div>
              <Input
                  type="text"
                  placeholder="Search reagents by name, CAS, formula..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  style={{ paddingLeft: '44px' }}
              />
            </div>
            <Button variant="secondary" onClick={() => setShowFilters(!showFilters)} icon={<FilterIcon size={16} />}>
              {showFilters ? 'Hide' : 'Filters'}
              {activeFiltersCount > 0 && (
                  <span style={{ marginLeft: '6px', background: 'linear-gradient(135deg, #3182ce, #38a169)', color: 'white', borderRadius: '10px', padding: '2px 8px', fontSize: '0.7rem', fontWeight: '700' }}>
                {activeFiltersCount}
              </span>
              )}
            </Button>
            {activeFiltersCount > 0 && <Button variant="ghost" onClick={clearFilters} icon={<CloseIcon size={16} />}>Clear</Button>}
          </div>

          {showFilters && (
              <div style={{ backgroundColor: '#fff', padding: '20px', borderRadius: '12px', marginBottom: '16px', border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(26, 54, 93, 0.06)' }}>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))', gap: '16px' }}>
                  <div>
                    <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Status</label>
                    <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
                      <option value="">All Statuses</option>
                      <option value="active">Active</option>
                      <option value="inactive">Inactive</option>
                    </Select>
                  </div>
                  <div>
                    <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Manufacturer</label>
                    <Select value={manufacturerFilter} onChange={(e) => setManufacturerFilter(e.target.value)}>
                      <option value="">All Manufacturers</option>
                      {manufacturers.map(m => <option key={m} value={m}>{m}</option>)}
                    </Select>
                  </div>
                  <div>
                    <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Sort By</label>
                    <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
                      <option value="created_at">Date Added</option>
                      <option value="name">Name</option>
                      <option value="total_quantity">Total Quantity</option>
                    </Select>
                  </div>
                  <div>
                    <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Sort Order</label>
                    <Select value={sortOrder} onChange={(e) => setSortOrder(e.target.value)}>
                      <option value="desc">Descending</option>
                      <option value="asc">Ascending</option>
                    </Select>
                  </div>
                </div>
              </div>
          )}
        </div>

        <div style={{ marginBottom: '15px', color: '#718096', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>
          Showing <strong style={{ color: '#1a365d' }}>{reagents.length}</strong> of <strong style={{ color: '#1a365d' }}>{pagination.total}</strong> reagents
        </span>
        </div>

        <div className="reagents-content">
          <div style={{ display: 'flex', padding: '14px 18px', background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.08) 0%, rgba(56, 161, 105, 0.08) 100%)', borderRadius: '12px 12px 0 0', fontWeight: '700', fontSize: '0.7rem', color: '#1a365d', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '2px', gap: '14px' }}>
            <div style={{ width: COLUMN_WIDTHS.expandIcon, flexShrink: 0 }}></div>
            <div style={{ flex: 1, display: 'grid', gridTemplateColumns: COLUMN_WIDTHS.gridColumns, gap: '12px' }}>
              <div>Name</div><div>Formula</div><div>MW</div><div>CAS</div><div>Status</div><div>Stock</div>
            </div>
            <div style={{ width: COLUMN_WIDTHS.actions, flexShrink: 0 }}>Actions</div>
          </div>

          {reagents.length === 0 && !loading ? (
              <div style={{ textAlign: 'center', padding: '60px 40px', color: '#718096', backgroundColor: '#fff', borderRadius: '0 0 12px 12px', border: '1px solid #e2e8f0', borderTop: 'none' }}>
                <div style={{ width: '80px', height: '80px', margin: '0 auto 20px', background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.1) 0%, rgba(56, 161, 105, 0.1) 100%)', borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <FlaskIcon size={32} color="#3182ce" />
                </div>
                <h3 style={{ margin: '0 0 8px 0', color: '#1a365d', fontSize: '1.1rem' }}>
                  {activeFiltersCount > 0 ? 'No matches found' : 'No reagents found'}
                </h3>
                <p style={{ margin: '0 0 20px 0', fontSize: '0.9rem' }}>
                  {activeFiltersCount > 0 ? "Try adjusting your filters." : "Get started by adding your first reagent."}
                </p>
              </div>
          ) : (
              reagents.map(reagent => (
                  <ReagentAccordionItem
                      key={reagent.id}
                      reagent={reagent}
                      isExpanded={expandedReagents.has(reagent.id)}
                      onToggle={() => toggleAccordion(reagent.id)}
                      onAction={handleAction}
                      onReagentsRefresh={refresh}
                      canEdit={canEditReagents()}
                      canDelete={isAdmin}
                  />
              ))
          )}

          {loading && <div style={{ padding: '20px', textAlign: 'center' }}><Loading /></div>}

          {/* --- ПАНЕЛЬ ПАГИНАЦИИ С ВЫБОРОМ КОЛИЧЕСТВА СТРОК --- */}
          {!loading && reagents.length > 0 && (
              <div style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '16px',
                marginTop: '20px',
                background: 'white',
                borderRadius: '8px',
                border: '1px solid #e2e8f0'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                  <div style={{ fontSize: '0.9rem', color: '#4a5568' }}>
                    Page <b>{pagination.page}</b> of <b>{pagination.totalPages}</b>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', borderLeft: '1px solid #e2e8f0', paddingLeft: '16px' }}>
                    <span style={{ fontSize: '0.8rem', color: '#718096' }}>Rows:</span>
                    <div style={{ width: '80px' }}>
                      <Select
                          value={pagination.perPage}
                          onChange={(e) => pagination.setPerPage(e.target.value)}
                          options={[
                            { value: 10, label: '10' },
                            { value: 20, label: '20' },
                            { value: 50, label: '50' },
                            { value: 100, label: '100' }
                          ]}
                      />
                    </div>
                  </div>
                </div>

                <div style={{ display: 'flex', gap: '8px' }}>
                  <Button
                      variant="outline"
                      onClick={pagination.goPrev}
                      disabled={pagination.page <= 1}
                      icon={<ChevronLeftIcon />}
                  >
                    Previous
                  </Button>
                  <Button
                      variant="outline"
                      onClick={pagination.goNext}
                      disabled={!pagination.hasNext && pagination.page >= pagination.totalPages}
                  >
                    Next <ChevronRightIcon />
                  </Button>
                </div>
              </div>
          )}
        </div>

        {showCreateModal && <CreateReagentModal isOpen={showCreateModal} onClose={() => setShowCreateModal(false)} onSave={handleCreateSuccess} />}
        {showEditModal && selectedReagent && <EditReagentModal isOpen={showEditModal} reagent={selectedReagent} onClose={() => { setShowEditModal(false); setSelectedReagent(null); }} onSave={handleEditSuccess} />}
        {showViewModal && selectedReagent && <ViewReagentModal isOpen={showViewModal} onClose={() => { setShowViewModal(false); setSelectedReagent(null); }} reagent={selectedReagent} />}
        {showImportModal && <BatchImportModal isOpen={showImportModal} onClose={() => setShowImportModal(false)} onImport={handleImport} existingReagents={reagents} existingBatches={allBatches} />}
      </div>
  );
};

export default Reagents;