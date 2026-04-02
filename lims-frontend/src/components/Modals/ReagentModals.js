// components/modals/ReagentModals.js

import React, { useState, useEffect, useCallback } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Select from '../Select';
import TextArea from '../TextArea';
import Button from '../Button';
import FormGroup from '../FormGroup';
import {
  CheckIcon,
  CloseIcon,
  AlertCircleIcon,
  FlaskIcon,
  DatabaseIcon,
  PlusIcon,
  EditIcon,
  TrashIcon,
  ClockIcon
} from '../Icons';
import { styles } from './styles';
import { useFormSubmit, cleanPayload, cleanPayloadForUpdate } from './helpers';
import { HazardSelect, HazardDisplay } from './HazardComponents';
import { PrintStickerModal, PrinterIcon } from './PrintComponents';
import { CreateBatchModal, EditBatchModal } from './BatchModals';
import { UsageHistoryModal } from './UsageHistoryModal';
import { useRooms } from '../hooks/useRooms';

// ==================== CreateReagentModal (with first batch) ====================

export const CreateReagentModal = ({ isOpen, onClose, onSave }) => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [reagentData, setReagentData] = useState({
    name: '', formula: '', molecular_weight: '', cas_number: '',
    manufacturer: '', physical_state: '',
    status: 'active', description: '',
    storage_conditions: '', appearance: '', hazard_pictograms: ''
  });
  const [batchData, setBatchData] = useState({
    batch_number: '', quantity: '', unit: 'g', pack_size: '', expiry_date: '', notes: ''
  });

  const validate = () => {
    if (!reagentData.name) { setError('Please specify the reagent name'); return false; }
    if (!batchData.batch_number) { setError('Please specify the batch number'); return false; }
    if (!batchData.quantity) { setError('Please specify the quantity'); return false; }
    return true;
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!validate()) return;
    setLoading(true); setError('');

    try {
      const reagentPayload = cleanPayload({ 
        ...reagentData, 
        molecular_weight: reagentData.molecular_weight ? parseFloat(reagentData.molecular_weight) : null 
      });
      reagentPayload.hazard_pictograms = reagentData.hazard_pictograms || '';
      const reagentResponse = await api.createReagent(reagentPayload);
      const newReagentId = reagentResponse.data?.id || reagentResponse.id;
      if (!newReagentId) throw new Error("Reagent ID not returned");

      const batchPayload = cleanPayload({ 
        ...batchData, 
        quantity: parseFloat(batchData.quantity),
        pack_size: batchData.pack_size ? parseFloat(batchData.pack_size) : null
      });
      if (batchPayload.expiry_date) batchPayload.expiry_date = `${batchPayload.expiry_date}T00:00:00Z`;
      await api.createBatch(newReagentId, batchPayload);

      onSave(); 
      onClose();
    } catch (err) { 
      console.error(err); 
      setError(err.message || 'Creation error'); 
    } finally { 
      setLoading(false); 
    }
  };

  const handleReagentChange = (e) => setReagentData({ ...reagentData, [e.target.name]: e.target.value });
  const handleBatchChange = (e) => setBatchData({ ...batchData, [e.target.name]: e.target.value });

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="New Reagent" size="lg">
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} color="#c53030" />
          {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem' }}>
          {/* Left: Reagent Info */}
          <div>
            <div style={styles.sectionTitle}>
              <FlaskIcon size={16} color="#3182ce" />Reagent info
            </div>
            <FormGroup label="Name" required>
              <Input 
                name="name" 
                value={reagentData.name} 
                onChange={handleReagentChange} 
                placeholder="Sodium Chloride" 
                required 
              />
            </FormGroup>
            <div style={styles.twoColGrid}>
              <FormGroup label="Formula">
                <Input 
                  name="formula" 
                  value={reagentData.formula} 
                  onChange={handleReagentChange} 
                  placeholder="NaCl" 
                />
              </FormGroup>
              <FormGroup label="CAS №">
                <Input 
                  name="cas_number" 
                  value={reagentData.cas_number} 
                  onChange={handleReagentChange} 
                  placeholder="7647-14-5" 
                />
              </FormGroup>
            </div>
            <div style={styles.twoColGrid}>
              <FormGroup label="Molecular Weight (g/mol)">
                <Input 
                  type="number" 
                  step="0.01" 
                  name="molecular_weight" 
                  value={reagentData.molecular_weight} 
                  onChange={handleReagentChange} 
                />
              </FormGroup>
              <FormGroup label="Physical State">
                <Select name="physical_state" value={reagentData.physical_state} onChange={handleReagentChange}>
                  <option value="">—</option>
                  <option value="solid">Solid</option>
                  <option value="liquid">Liquid</option>
                  <option value="gas">Gas</option>
                  <option value="powder">Powder</option>
                  <option value="crystal">Crystal</option>
                  <option value="solution">Solution</option>
                </Select>
              </FormGroup>
            </div>
            <FormGroup label="Manufacturer">
              <Input 
                name="manufacturer" 
                value={reagentData.manufacturer} 
                onChange={handleReagentChange} 
                placeholder="e.g. Sigma-Aldrich" 
              />
            </FormGroup>
            <div style={styles.twoColGrid}>
              <FormGroup label="Storage Conditions">
                <Input 
                  name="storage_conditions" 
                  value={reagentData.storage_conditions} 
                  onChange={handleReagentChange} 
                  placeholder="+4°C" 
                />
              </FormGroup>
              <FormGroup label="Appearance">
                <Input 
                  name="appearance" 
                  value={reagentData.appearance} 
                  onChange={handleReagentChange} 
                  placeholder="White crystalline powder" 
                />
              </FormGroup>
            </div>
            <FormGroup label="Hazard Pictograms">
              <HazardSelect 
                selectedCodes={reagentData.hazard_pictograms} 
                onChange={(val) => setReagentData({ ...reagentData, hazard_pictograms: val })} 
              />
            </FormGroup>
          </div>

          {/* Right: First Batch */}
          <div>
            <div style={styles.sectionTitle}>
              <DatabaseIcon size={16} color="#3182ce" />First Batch
            </div>
            <div style={styles.card}>
              <FormGroup label="Batch Number / Lot" required>
                <Input 
                  name="batch_number" 
                  value={batchData.batch_number} 
                  onChange={handleBatchChange} 
                  placeholder="LOT-2024-001" 
                  required 
                />
              </FormGroup>
              <div style={styles.twoColGrid}>
                <FormGroup label="Quantity" required>
                  <Input 
                    type="number" 
                    step="0.01" 
                    name="quantity" 
                    value={batchData.quantity} 
                    onChange={handleBatchChange} 
                    required 
                  />
                </FormGroup>
                <FormGroup label="Unit" required>
                  <Select name="unit" value={batchData.unit} onChange={handleBatchChange}>
                    <option value="g">g</option>
                    <option value="mg">mg</option>
                    <option value="kg">kg</option>
                    <option value="mL">mL</option>
                    <option value="L">L</option>
                    <option value="pcs">pcs</option>
                  </Select>
                </FormGroup>
              </div>
              <FormGroup label="Pack Size" hint="Amount per pack (for counting packs)">
                <Input 
                  type="number" 
                  step="any"
                  min="0"
                  name="pack_size" 
                  value={batchData.pack_size} 
                  onChange={handleBatchChange}
                  placeholder="e.g. 100 for 100g packs"
                />
              </FormGroup>
              <FormGroup label="Expiry Date">
                <Input 
                  type="date" 
                  name="expiry_date" 
                  value={batchData.expiry_date} 
                  onChange={handleBatchChange} 
                />
              </FormGroup>
              <FormGroup label="Notes">
                <TextArea 
                  name="notes" 
                  value={batchData.notes} 
                  onChange={handleBatchChange} 
                  rows={2} 
                />
              </FormGroup>
            </div>
          </div>
        </div>
        <div style={styles.buttonContainer}>
          <Button variant="secondary" type="button" onClick={onClose} icon={<CloseIcon size={16} />}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" loading={loading} icon={<CheckIcon size={16} />}>
            Create Reagent
          </Button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== EditReagentModal ====================

export const EditReagentModal = ({ isOpen, onClose, reagent, onSave }) => {
  const [formData, setFormData] = useState({
    name: '', formula: '', molecular_weight: '', cas_number: '',
    manufacturer: '', physical_state: '',
    status: 'active', description: '',
    storage_conditions: '', appearance: '', hazard_pictograms: ''
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (reagent) {
      setFormData({
        name: reagent.name || '',
        formula: reagent.formula || '',
        molecular_weight: reagent.molecular_weight || '',
        cas_number: reagent.cas_number || '',        
        manufacturer: reagent.manufacturer || '',
        physical_state: reagent.physical_state || '',
        status: reagent.status || 'active',
        description: reagent.description || '',
        storage_conditions: reagent.storage_conditions || '',
        appearance: reagent.appearance || '',
        hazard_pictograms: reagent.hazard_pictograms || ''
      });
    }
  }, [reagent]);

  const handleSubmit = useFormSubmit(async () => {
    setLoading(true);
    try {
      const payload = cleanPayloadForUpdate(formData);
      if (payload.molecular_weight) payload.molecular_weight = parseFloat(payload.molecular_weight);
      // Explicitly preserve hazard_pictograms (empty string = clear all)
      if (formData.hazard_pictograms === '') payload.hazard_pictograms = '';
      
      const response = await api.updateReagent(reagent.id, payload);
      if (response && response.success !== false) { 
        onSave(); 
        onClose(); 
      } else { 
        setError(response?.message || 'Update error'); 
      }
    } catch (err) { 
      setError(err.message); 
    } finally { 
      setLoading(false); 
    }
  }, () => formData.name);

  const handleChange = (e) => setFormData({ ...formData, [e.target.name]: e.target.value });

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Edit Reagent">
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} color="#c53030" />
          {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={styles.formGrid}>
          <FormGroup label="Name" required>
            <Input name="name" value={formData.name} onChange={handleChange} required />
          </FormGroup>
          <div style={styles.threeColGrid}>
            <FormGroup label="Formula">
              <Input name="formula" value={formData.formula} onChange={handleChange} />
            </FormGroup>
            <FormGroup label="CAS №">
              <Input name="cas_number" value={formData.cas_number} onChange={handleChange} />
            </FormGroup>
            <FormGroup label="Molecular Weight (g/mol)">
              <Input 
                name="molecular_weight" 
                type="number" 
                step="0.01" 
                value={formData.molecular_weight} 
                onChange={handleChange} 
              />
            </FormGroup>
          </div>
          <div style={styles.twoColGrid}>
            <FormGroup label="Manufacturer">
              <Input name="manufacturer" value={formData.manufacturer} onChange={handleChange} placeholder="e.g. Sigma-Aldrich" />
            </FormGroup>
            <FormGroup label="Physical State">
              <Select name="physical_state" value={formData.physical_state} onChange={handleChange}>
                <option value="">—</option>
                <option value="solid">Solid</option>
                <option value="liquid">Liquid</option>
                <option value="gas">Gas</option>
                <option value="powder">Powder</option>
                <option value="crystal">Crystal</option>
                <option value="solution">Solution</option>
              </Select>
            </FormGroup>
          </div>
          <div style={styles.twoColGrid}>
            <FormGroup label="Storage Conditions">
              <Input name="storage_conditions" value={formData.storage_conditions} onChange={handleChange} />
            </FormGroup>
            <FormGroup label="Appearance">
              <Input name="appearance" value={formData.appearance} onChange={handleChange} />
            </FormGroup>
          </div>
          <FormGroup label="Hazard Pictograms">
            <HazardSelect 
              selectedCodes={formData.hazard_pictograms} 
              onChange={(val) => setFormData({ ...formData, hazard_pictograms: val })} 
            />
          </FormGroup>
          <div style={styles.twoColGrid}>
            <FormGroup label="Status">
              <Select name="status" value={formData.status} onChange={handleChange}>
                <option value="active">Active</option>
                <option value="inactive">Inactive</option>
                <option value="discontinued">Discontinued</option>
              </Select>
            </FormGroup>
            <FormGroup label="Description">
              <TextArea name="description" value={formData.description} onChange={handleChange} rows={2} />
            </FormGroup>
          </div>
        </div>
        <div style={styles.buttonContainer}>
          <Button variant="secondary" onClick={onClose} icon={<CloseIcon size={16} />}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" loading={loading} icon={<CheckIcon size={16} />}>
            Save
          </Button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== View Reagent Modal (two-column layout) ====================

// --- Batch Mini Card for View Modal ---
const ViewBatchCard = ({ batch, reagent, rooms, isExpanded, onToggle, onAction, onRefresh }) => {
  const available = batch.quantity - (batch.reserved_quantity || 0);
  const containerCount = batch.container_count || batch.pack_count || 0;
  const openedCount = batch.opened_count || 0;
  const placedCount = batch.placed_count || 0;

  const getExpiryInfo = () => {
    if (!batch.expiry_date) return { text: '—', color: '#a0aec0' };
    const days = Math.ceil((new Date(batch.expiry_date) - new Date()) / 86400000);
    if (days < 0) return { text: `Expired ${Math.abs(days)}d ago`, color: '#e53e3e' };
    if (days <= 30) return { text: `${days}d left`, color: '#dd6b20' };
    return { text: new Date(batch.expiry_date).toLocaleDateString(), color: '#718096' };
  };

  const expiry = getExpiryInfo();
  const statusColors = {
    available: { bg: '#c6f6d5', text: '#22543d' },
    low_stock: { bg: '#fefcbf', text: '#744210' },
    depleted: { bg: '#fed7d7', text: '#822727' },
    expired: { bg: '#fed7d7', text: '#822727' },
  };
  const sc = statusColors[batch.status] || { bg: '#edf2f7', text: '#4a5568' };

  return (
    <div style={{
      border: isExpanded ? '1px solid #3182ce' : '1px solid #e2e8f0',
      borderRadius: '10px',
      overflow: 'hidden',
      backgroundColor: '#fff',
      transition: 'all 0.15s ease',
    }}>
      {/* Compact header — always visible */}
      <div
        onClick={onToggle}
        style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          padding: '10px 14px', cursor: 'pointer',
          backgroundColor: isExpanded ? '#f7faff' : '#fff',
          transition: 'background 0.15s',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flex: 1, minWidth: 0 }}>
          <span style={{ fontWeight: '700', fontSize: '0.9rem', color: '#1a365d', whiteSpace: 'nowrap' }}>
            {batch.batch_number}
          </span>
          <span style={{
            fontSize: '0.8rem', fontWeight: '600',
            color: available > 0 ? '#2d3748' : '#e53e3e',
          }}>
            {available} {batch.unit}
          </span>
          <span style={{
            fontSize: '0.7rem', padding: '2px 8px', borderRadius: '4px',
            backgroundColor: sc.bg, color: sc.text, fontWeight: '600',
          }}>
            {batch.status}
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
          {containerCount > 0 && (
            <span style={{ fontSize: '0.75rem', color: '#718096' }}>
              📦{containerCount} {openedCount > 0 && `(${openedCount}⊙)`}
              {placedCount < containerCount && (
                <span style={{ color: '#e53e3e', marginLeft: '4px' }}>
                  {containerCount - placedCount} unplaced
                </span>
              )}
            </span>
          )}
          <span style={{ fontSize: '0.75rem', color: expiry.color, fontWeight: '500' }}>
            {expiry.text}
          </span>
          <span style={{
            fontSize: '10px', color: '#a0aec0',
            transform: isExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
            transition: 'transform 0.2s',
          }}>▼</span>
        </div>
      </div>

      {/* Expanded details */}
      {isExpanded && (
        <div style={{
          padding: '12px 14px', borderTop: '1px solid #e2e8f0',
          backgroundColor: '#fafbfc',
        }}>
          <div style={{
            display: 'grid', gridTemplateColumns: '1fr 1fr',
            gap: '4px 16px', fontSize: '0.8rem', marginBottom: '12px',
          }}>
            {batch.supplier && <DetailItem label="Supplier" value={batch.supplier} />}
            {(batch.manufacturer || reagent.manufacturer) && (
              <DetailItem label="Manufacturer" value={batch.manufacturer || reagent.manufacturer} />
            )}
            {batch.cat_number && <DetailItem label="Cat #" value={batch.cat_number} />}
            {batch.pack_size > 0 && <DetailItem label="Pack" value={`${batch.pack_size} ${batch.unit}`} />}
            {(batch.reserved_quantity || 0) > 0 && (
              <DetailItem label="Reserved" value={`${batch.reserved_quantity} ${batch.unit}`} color="#dd6b20" />
            )}
            {batch.received_date && (
              <DetailItem label="Received" value={new Date(batch.received_date).toLocaleDateString()} />
            )}
            {batch.notes && <DetailItem label="Notes" value={batch.notes} span />}
          </div>

          {/* Action buttons */}
          <div style={{ display: 'flex', gap: '6px', justifyContent: 'flex-end' }}>
            <Button size="small" variant="ghost" onClick={() => onAction('history', batch)}
              icon={<ClockIcon size={13} />}>History</Button>
            <Button size="small" variant="secondary" onClick={() => onAction('edit', batch)}
              icon={<EditIcon size={13} />}>Edit</Button>
            <Button size="small" variant="danger" onClick={() => onAction('delete', batch)}
              icon={<TrashIcon size={13} />}>Delete</Button>
          </div>
        </div>
      )}
    </div>
  );
};

const DetailItem = ({ label, value, color, span }) => (
  <div style={{ 
    display: 'flex', justifyContent: 'space-between', padding: '3px 0',
    gridColumn: span ? '1 / -1' : undefined,
  }}>
    <span style={{ color: '#718096' }}>{label}</span>
    <span style={{ color: color || '#1a365d', fontWeight: '500', textAlign: 'right', maxWidth: '65%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
      {value}
    </span>
  </div>
);

// --- Main ViewReagentModal ---
export const ViewReagentModal = ({ isOpen, onClose, reagent, onEdit }) => {
  const [batches, setBatches] = useState([]);
  const [, setLoading] = useState(false);
  const [expandedBatchId, setExpandedBatchId] = useState(null);
  const [showCreateBatch, setShowCreateBatch] = useState(false);
  const [showEditBatch, setShowEditBatch] = useState(false);
  const [showUsageHistory, setShowUsageHistory] = useState(false);
  const [showPrintModal, setShowPrintModal] = useState(false);
  const [selectedBatch, setSelectedBatch] = useState(null);
  const { rooms } = useRooms();

  const loadBatches = useCallback(async () => {
    if (!reagent?.id) return;
    setLoading(true);
    try {
      const response = await api.getReagentBatches(reagent.id);
      let batchData = response;
      if (response && typeof response === 'object' && !Array.isArray(response)) {
        batchData = response.data || response;
        if (batchData && typeof batchData === 'object' && !Array.isArray(batchData)) {
          batchData = batchData.data || [];
        }
      }
      setBatches(Array.isArray(batchData) ? batchData : []);
    } catch (err) {
      console.error('Failed to load batches:', err);
      setBatches([]);
    } finally {
      setLoading(false);
    }
  }, [reagent?.id]);

  useEffect(() => {
    if (isOpen && reagent?.id) loadBatches();
  }, [isOpen, reagent?.id, loadBatches]);

  const handleBatchAction = async (action, item) => {
    if (action === 'history') { setSelectedBatch(item); setShowUsageHistory(true); }
    else if (action === 'edit') { setSelectedBatch(item); setShowEditBatch(true); }
    else if (action === 'print') { setSelectedBatch(item); setShowPrintModal(true); }
    else if (action === 'delete') {
      if (window.confirm(`Delete batch "${item.batch_number}"?`)) {
        await api.deleteBatch(item.reagent_id, item.id);
        loadBatches();
      }
    }
  };

  if (!isOpen || !reagent) return null;

  // Summary stats
  const totalQty = batches.reduce((s, b) => s + (b.quantity || 0), 0);
  const primaryUnit = batches[0]?.unit || '';
  const availableCount = batches.filter(b => b.status === 'available').length;
  const expiringCount = batches.filter(b => {
    if (!b.expiry_date) return false;
    const days = Math.ceil((new Date(b.expiry_date) - new Date()) / 86400000);
    return days >= 0 && days <= 30;
  }).length;

  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: 'rgba(0,0,0,0.5)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 2000,
    }}>
      <div style={{
        backgroundColor: 'white', borderRadius: '14px',
        maxWidth: '1060px', width: '95%', maxHeight: '90vh',
        display: 'flex', flexDirection: 'column',
        boxShadow: '0 24px 48px rgba(0,0,0,0.18)',
        overflow: 'hidden',
      }}>
        {/* ===== Header ===== */}
        <div style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          padding: '1.25rem 1.5rem',
          borderBottom: '2px solid #e2e8f0',
          flexShrink: 0,
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <h2 style={{ margin: 0, fontSize: '1.2rem', color: '#1a365d' }}>{reagent.name}</h2>
            {reagent.formula && (
              <span style={{
                backgroundColor: '#edf2f7', padding: '3px 10px',
                borderRadius: '6px', fontFamily: 'monospace', fontSize: '0.85rem', color: '#4a5568',
              }}>{reagent.formula}</span>
            )}
            {onEdit && (
              <Button size="sm" variant="secondary" onClick={() => onEdit(reagent)} icon={<EditIcon size={14} />}>
                Edit
              </Button>
            )}
          </div>
          <button
            onClick={onClose}
            style={{ border: 'none', background: 'none', fontSize: '1.5rem', cursor: 'pointer', color: '#a0aec0' }}
          >×</button>
        </div>

        {/* ===== Two-Column Body ===== */}
        <div style={{
          display: 'grid', gridTemplateColumns: '340px 1fr',
          flex: 1, overflow: 'hidden', minHeight: 0,
        }}>
          {/* --- Left: Reagent Info --- */}
          <div style={{
            padding: '1.25rem', overflowY: 'auto',
            borderRight: '1px solid #e2e8f0', backgroundColor: '#f8fafc',
          }}>
            <div style={{ fontSize: '0.7rem', fontWeight: '700', color: '#718096', textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: '10px' }}>
              🧪 Reagent Properties
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', fontSize: '0.85rem' }}>
              <InfoRow label="CAS №" value={reagent.cas_number} mono />
              <InfoRow label="Mol. Mass" value={reagent.molecular_weight ? `${reagent.molecular_weight} g/mol` : null} />
              <InfoRow label="Physical State" value={reagent.physical_state} />
              <InfoRow label="Appearance" value={reagent.appearance} />
              <InfoRow label="Manufacturer" value={reagent.manufacturer} />
              <InfoRow label="Storage" value={reagent.storage_conditions} />
              {reagent.description && <InfoRow label="Description" value={reagent.description} />}
              <InfoRow label="Status" value={reagent.status} badge />
              <div style={{ marginTop: '4px' }}>
                <div style={{ fontSize: '0.7rem', color: '#718096', marginBottom: '4px' }}>Hazards</div>
                <HazardDisplay codes={reagent.hazard_pictograms} />
              </div>
            </div>

            {/* Stock Summary */}
            <div style={{
              marginTop: '20px', padding: '12px',
              backgroundColor: '#fff', borderRadius: '10px', border: '1px solid #e2e8f0',
            }}>
              <div style={{ fontSize: '0.7rem', fontWeight: '700', color: '#718096', textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: '8px' }}>
                📊 Stock Summary
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
                <MiniStat label="Total" value={`${totalQty.toFixed(1)} ${primaryUnit}`} color="#1a365d" />
                <MiniStat label="Batches" value={batches.length} color="#3182ce" />
                <MiniStat label="Available" value={availableCount} color="#38a169" />
                {expiringCount > 0 && <MiniStat label="Expiring" value={expiringCount} color="#dd6b20" />}
              </div>
            </div>
          </div>

          {/* --- Right: Batches --- */}
          <div style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            {/* Batches header */}
            <div style={{
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
              padding: '12px 16px', borderBottom: '1px solid #edf2f7', flexShrink: 0,
            }}>
              <span style={{ fontSize: '0.9rem', fontWeight: '700', color: '#1a365d', display: 'flex', alignItems: 'center', gap: '6px' }}>
                <DatabaseIcon size={16} color="#3182ce" /> Batches ({batches.length})
              </span>
              <div style={{ display: 'flex', gap: '6px' }}>
                {batches.length > 0 && (
                  <Button size="small" variant="secondary"
                    onClick={() => { setSelectedBatch(null); setShowPrintModal(true); }}
                    icon={<PrinterIcon size={13} />}>Print</Button>
                )}
                <Button size="small" variant="primary"
                  onClick={() => setShowCreateBatch(true)}
                  icon={<PlusIcon size={14} />}>Add Batch</Button>
              </div>
            </div>

            {/* Scrollable batch list */}
            <div style={{ flex: 1, overflowY: 'auto', padding: '12px 16px' }}>
              {batches.length === 0 ? (
                <div style={{
                  textAlign: 'center', color: '#a0aec0', padding: '40px 20px',
                  fontSize: '0.9rem',
                }}>
                  <FlaskIcon size={28} color="#cbd5e0" style={{ marginBottom: '8px' }} />
                  <p style={{ margin: 0 }}>No batches found</p>
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                  {batches.map(batch => (
                    <ViewBatchCard
                      key={batch.id}
                      batch={batch}
                      reagent={reagent}
                      rooms={rooms}
                      isExpanded={expandedBatchId === batch.id}
                      onToggle={() => setExpandedBatchId(prev => prev === batch.id ? null : batch.id)}
                      onAction={handleBatchAction}
                      onRefresh={loadBatches}
                    />
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ===== Footer ===== */}
        <div style={{
          padding: '12px 1.5rem', borderTop: '1px solid #e2e8f0',
          display: 'flex', justifyContent: 'flex-end', flexShrink: 0,
        }}>
          <Button onClick={onClose} variant="secondary" icon={<CloseIcon size={16} />}>Close</Button>
        </div>

        {/* Sub-modals */}
        {showCreateBatch && (
          <CreateBatchModal
            isOpen={showCreateBatch}
            reagentId={reagent.id}
            onClose={() => setShowCreateBatch(false)}
            onSave={() => { setShowCreateBatch(false); loadBatches(); }}
          />
        )}
        {showEditBatch && selectedBatch && (
          <EditBatchModal
            isOpen={showEditBatch}
            reagentId={reagent.id}
            batch={selectedBatch}
            onClose={() => setShowEditBatch(false)}
            onSave={() => { setShowEditBatch(false); setSelectedBatch(null); loadBatches(); }}
          />
        )}
        {showUsageHistory && selectedBatch && (
          <UsageHistoryModal
            isOpen={showUsageHistory}
            onClose={() => setShowUsageHistory(false)}
            reagentId={reagent.id}
            batchId={selectedBatch.id}
            batch={selectedBatch}
          />
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
    </div>
  );
};

// --- Helpers for ViewReagentModal ---
const InfoRow = ({ label, value, mono, badge }) => {
  if (!value && value !== 0) return (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 0' }}>
      <span style={{ color: '#a0aec0', fontSize: '0.8rem' }}>{label}</span>
      <span style={{ color: '#cbd5e0', fontSize: '0.8rem' }}>—</span>
    </div>
  );
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 0', borderBottom: '1px solid #f0f4f8' }}>
      <span style={{ color: '#718096', fontSize: '0.8rem' }}>{label}</span>
      {badge ? (
        <span style={{
          fontSize: '0.75rem', padding: '1px 8px', borderRadius: '4px',
          backgroundColor: value === 'active' ? '#c6f6d5' : '#fefcbf',
          color: value === 'active' ? '#22543d' : '#744210',
          fontWeight: '600',
        }}>{value}</span>
      ) : (
        <span style={{
          color: '#1a365d', fontWeight: '500', fontSize: '0.8rem',
          fontFamily: mono ? 'monospace' : 'inherit',
          textAlign: 'right', maxWidth: '60%',
          overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>{value}</span>
      )}
    </div>
  );
};

const MiniStat = ({ label, value, color }) => (
  <div style={{ textAlign: 'center', padding: '6px 4px' }}>
    <div style={{ fontSize: '1.1rem', fontWeight: '700', color }}>{value}</div>
    <div style={{ fontSize: '0.65rem', color: '#a0aec0', textTransform: 'uppercase', letterSpacing: '0.05em' }}>{label}</div>
  </div>
);

export default {
  CreateReagentModal,
  EditReagentModal,
  ViewReagentModal
};