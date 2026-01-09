// components/modals/ReagentModals.js

import React, { useState, useEffect, useCallback } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Select from '../Select';
import TextArea from '../TextArea';
import Button from '../Button';
import FormGroup from '../FormGroup';
import Table from '../Table';
import { 
  CheckIcon, 
  CloseIcon, 
  AlertCircleIcon, 
  FlaskIcon,
  DatabaseIcon,
  PlusIcon,
  EditIcon,
  TrashIcon 
} from '../Icons';
import { styles } from './styles';
import { useFormSubmit, cleanPayload } from './helpers';
import { HazardSelect, HazardDisplay } from './HazardComponents';
import { PrintStickerModal, PrinterIcon } from './PrintComponents';
import { CreateBatchModal, EditBatchModal } from './BatchModals';
import { UsageHistoryModal } from './UsageHistoryModal';

// ==================== CreateReagentModal (with first batch) ====================

export const CreateReagentModal = ({ isOpen, onClose, onSave }) => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [reagentData, setReagentData] = useState({
    name: '', formula: '', molecular_weight: '', cas_number: '',
    manufacturer: '', status: 'active', description: '',
    storage_conditions: '', appearance: '', hazard_pictograms: ''
  });
  const [batchData, setBatchData] = useState({
    batch_number: '', quantity: '', unit: 'g', expiry_date: '', location: '', notes: ''
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
        manufacturer: reagentData.manufacturer 
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
              <FormGroup label="Manufacturer">
                <Input 
                  name="manufacturer" 
                  value={reagentData.manufacturer} 
                  onChange={handleReagentChange} 
                />
              </FormGroup>
            </div>
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
                    <option value="kg">kg</option>
                    <option value="ml">ml</option>
                    <option value="L">L</option>
                    <option value="pcs">pcs</option>
                  </Select>
                </FormGroup>
              </div>
              <div style={styles.twoColGrid}>
                <FormGroup label="Expiry Date">
                  <Input 
                    type="date" 
                    name="expiry_date" 
                    value={batchData.expiry_date} 
                    onChange={handleBatchChange} 
                  />
                </FormGroup>
                <FormGroup label="Location">
                  <Input 
                    name="location" 
                    value={batchData.location} 
                    onChange={handleBatchChange} 
                    placeholder="Shelf A-2" 
                  />
                </FormGroup>
              </div>
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
    manufacturer: '', status: 'active', description: '',
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
      const payload = cleanPayload(formData);
      if (payload.molecular_weight) payload.molecular_weight = parseFloat(payload.molecular_weight);
      payload.hazard_pictograms = formData.hazard_pictograms || '';
      
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
          <div style={styles.threeColGrid}>
            <FormGroup label="Manufacturer">
              <Input name="manufacturer" value={formData.manufacturer} onChange={handleChange} />
            </FormGroup>
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

// ==================== View Reagent Modal ====================

export const ViewReagentModal = ({ isOpen, onClose, reagent, onEdit }) => {
  const [batches, setBatches] = useState([]);
  const [loading, setLoading] = useState(false);
  const [showCreateBatch, setShowCreateBatch] = useState(false);
  const [showEditBatch, setShowEditBatch] = useState(false);
  const [showUsageHistory, setShowUsageHistory] = useState(false);
  const [showPrintModal, setShowPrintModal] = useState(false);
  const [selectedBatch, setSelectedBatch] = useState(null);

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

  const handleCreateBatchSuccess = () => { 
    setShowCreateBatch(false); 
    loadBatches(); 
  };
  
  const handleEditBatchSuccess = () => { 
    setShowEditBatch(false); 
    setSelectedBatch(null); 
    loadBatches(); 
  };
  
  const handleBatchAction = async (action, item) => {
    if (action === 'view') { 
      setSelectedBatch(item); 
      setShowUsageHistory(true); 
    } else if (action === 'edit') { 
      setSelectedBatch(item); 
      setShowEditBatch(true); 
    } else if (action === 'print') { 
      setSelectedBatch(item); 
      setShowPrintModal(true); 
    } else if (action === 'delete') {
      if (window.confirm(`Delete batch "${item.batch_number}"?`)) {
        await api.deleteBatch(item.reagent_id, item.id);
        loadBatches();
      }
    }
  };

  const handlePrintAll = () => {
    setSelectedBatch(null);
    setShowPrintModal(true);
  };

  if (!isOpen || !reagent) return null;

  return (
    <div style={{ 
      position: 'fixed', 
      top: 0, 
      left: 0, 
      right: 0, 
      bottom: 0, 
      backgroundColor: 'rgba(0,0,0,0.5)', 
      display: 'flex', 
      alignItems: 'center', 
      justifyContent: 'center', 
      zIndex: 2000 
    }}>
      <div style={{ 
        backgroundColor: 'white', 
        borderRadius: '12px', 
        padding: '1.5rem', 
        maxWidth: '950px', 
        width: '95%', 
        maxHeight: '90vh', 
        overflowY: 'auto', 
        boxShadow: '0 20px 40px rgba(0,0,0,0.2)' 
      }}>
        {/* Header */}
        <div style={{ 
          display: 'flex', 
          justifyContent: 'space-between', 
          alignItems: 'center', 
          marginBottom: '1.5rem', 
          paddingBottom: '1rem', 
          borderBottom: '2px solid #e2e8f0' 
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <h2 style={{ margin: 0, fontSize: '1.25rem' }}>{reagent.name}</h2>
            {reagent.formula && (
              <span style={{ 
                backgroundColor: '#edf2f7', 
                padding: '0.25rem 0.75rem', 
                borderRadius: '4px', 
                fontFamily: 'monospace', 
                fontSize: '0.9rem' 
              }}>
                {reagent.formula}
              </span>
            )}
            {onEdit && (
              <Button size="sm" variant="secondary" onClick={() => onEdit(reagent)} icon={<EditIcon size={14} />}>
                Edit
              </Button>
            )}
          </div>
          <button 
            onClick={onClose} 
            style={{ 
              border: 'none', 
              background: 'none', 
              fontSize: '1.5rem', 
              cursor: 'pointer', 
              color: '#a0aec0' 
            }}
          >
            ×
          </button>
        </div>

        {/* Info Grid */}
        <div style={{ 
          display: 'grid', 
          gridTemplateColumns: '1fr 1fr', 
          gap: '1rem', 
          marginBottom: '1.5rem', 
          padding: '1.25rem', 
          backgroundColor: '#f7fafc', 
          borderRadius: '8px', 
          border: '1px solid #e2e8f0' 
        }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Formula</div>
              <div style={{ fontWeight: '500', fontFamily: 'monospace', fontSize: '1rem' }}>
                {reagent.formula || '—'}
              </div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>CAS №</div>
              <div style={{ fontWeight: '500' }}>{reagent.cas_number || '—'}</div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Mol. mass</div>
              <div style={{ fontWeight: '500' }}>
                {reagent.molecular_weight ? `${reagent.molecular_weight} g/mol` : '—'}
              </div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Manufacturer</div>
              <div style={{ fontWeight: '500' }}>{reagent.manufacturer || '—'}</div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Hazard</div>
              <HazardDisplay codes={reagent.hazard_pictograms} />
            </div>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Appearance</div>
              <div style={{ fontWeight: '500' }}>{reagent.appearance || '—'}</div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Storage conditions</div>
              <div style={{ fontWeight: '500' }}>{reagent.storage_conditions || '—'}</div>
            </div>
            {reagent.description && (
              <div>
                <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>Description</div>
                <div style={{ fontWeight: '500' }}>{reagent.description}</div>
              </div>
            )}
          </div>
        </div>

        {/* Batches Section */}
        <div style={styles.sectionHeader}>
          <h3 style={{ margin: 0, fontSize: '1rem' }}>📦 Batches in stock</h3>
          <div style={{ display: 'flex', gap: '8px' }}>
            {batches.length > 0 && (
              <Button variant="secondary" size="sm" onClick={handlePrintAll} icon={<PrinterIcon size={14} />}>
                Print Stickers
              </Button>
            )}
            <Button variant="primary" size="sm" onClick={() => setShowCreateBatch(true)} icon={<PlusIcon size={14} />}>
              Add batch
            </Button>
          </div>
        </div>

        <Table
          data={batches}
          columns={[
            { key: 'batch_number', label: 'Batch' },
            { 
              key: 'quantity', 
              label: 'Quantity', 
              render: i => `${i.quantity ?? 0} ${i.unit || ''}` 
            },
            { 
              key: 'reserved_quantity', 
              label: 'Reserved', 
              render: i => {
                const reserved = i.reserved_quantity || 0;
                return reserved > 0 
                  ? <span style={{ color: '#dd6b20', fontWeight: '500' }}>{reserved} {i.unit || ''}</span> 
                  : <span style={{ color: '#a0aec0' }}>—</span>;
              }
            },
            { key: 'status', label: 'Status' },
            { 
              key: 'expiry_date', 
              label: 'Expiry Date', 
              render: i => i.expiry_date ? new Date(i.expiry_date).toLocaleDateString() : '—' 
            },
            { 
              key: 'storage_location', 
              label: 'Location', 
              render: i => i.storage_location || i.location || '—' 
            },
            {
              key: 'actions', 
              label: '', 
              render: i => (
                <div style={{ display: 'flex', gap: '4px' }}>
                  <Button size="sm" variant="secondary" onClick={() => handleBatchAction('view', i)} icon={<FlaskIcon size={12} />}>
                    Use
                  </Button>
                  <Button size="sm" variant="primary" onClick={() => handleBatchAction('edit', i)} icon={<EditIcon size={12} />} />
                  <Button size="sm" variant="danger" onClick={() => handleBatchAction('delete', i)} icon={<TrashIcon size={12} />} />
                </div>
              )
            }
          ]}
          emptyMessage="Batches not found"
        />

        <div style={{ ...styles.buttonContainer, marginTop: '1.5rem' }}>
          <Button onClick={onClose} variant="secondary" icon={<CloseIcon size={16} />}>
            Close
          </Button>
        </div>

        {/* Sub-modals */}
        {showCreateBatch && (
          <CreateBatchModal 
            isOpen={showCreateBatch} 
            reagentId={reagent.id} 
            onClose={() => setShowCreateBatch(false)} 
            onSave={handleCreateBatchSuccess} 
          />
        )}
        {showEditBatch && selectedBatch && (
          <EditBatchModal 
            isOpen={showEditBatch} 
            reagentId={reagent.id} 
            batch={selectedBatch} 
            onClose={() => setShowEditBatch(false)} 
            onSave={handleEditBatchSuccess} 
          />
        )}
        {showUsageHistory && selectedBatch && (
          <UsageHistoryModal 
            isOpen={showUsageHistory} 
            onClose={() => setShowUsageHistory(false)} 
            reagentId={reagent.id} 
            batchId={selectedBatch.id} 
            onSave={loadBatches} 
          />
        )}
        
        {/* Print Modal */}
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

export default {
  CreateReagentModal,
  EditReagentModal,
  ViewReagentModal
};
