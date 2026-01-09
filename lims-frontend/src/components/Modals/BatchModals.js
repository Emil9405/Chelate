// components/modals/BatchModals.js

import React, { useState, useEffect } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Select from '../Select';
import TextArea from '../TextArea';
import Button from '../Button';
import FormGroup from '../FormGroup';
import { CheckIcon, CloseIcon, AlertCircleIcon } from '../Icons';
import { styles } from './styles';
import { useFormSubmit, cleanPayload } from './helpers';

const BatchFormModal = ({ isOpen, onClose, title, reagentId, batch = null, onSave }) => {
  const isEdit = !!batch;
  const [formData, setFormData] = useState({
    batch_number: '', 
    quantity: '', 
    unit: 'g', 
    supplier: '', 
    manufacturer: '',
    received_date: '', 
    expiry_date: '', 
    location: '', 
    notes: ''
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (batch) {
      setFormData({
        batch_number: batch.batch_number || '',
        quantity: batch.quantity != null ? parseFloat(batch.quantity) : '',
        unit: batch.unit || 'g',
        supplier: batch.supplier || '',
        manufacturer: batch.manufacturer || '',
        received_date: batch.received_date?.split('T')[0] || '',
        expiry_date: batch.expiry_date?.split('T')[0] || '',
        location: batch.location || '',
        notes: batch.notes || ''
      });
    }
  }, [batch]);

  const handleSubmit = useFormSubmit(async () => {
    setLoading(true);
    const payload = cleanPayload({ 
      ...formData, 
      quantity: parseFloat(formData.quantity) 
    });
    // Format dates with timezone
    if (payload.received_date) payload.received_date = `${payload.received_date}T00:00:00Z`;
    if (payload.expiry_date) payload.expiry_date = `${payload.expiry_date}T00:00:00Z`;
    
    try {
      let response;
      if (isEdit) {
        response = await api.updateBatch(batch.reagent_id || reagentId, batch.id, payload);
      } else {
        response = await api.createBatch(reagentId, payload);
      }
      if (response && response.success !== false) { 
        onSave(); 
        onClose(); 
      } else { 
        setError(response?.message || 'Error'); 
      }
    } catch (err) { 
      setError(err.message); 
    } finally { 
      setLoading(false); 
    }
  }, () => formData.batch_number && formData.quantity);

  const handleChange = (e) => {
    setFormData(prev => ({ ...prev, [e.target.name]: e.target.value }));
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title}>
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} color="#c53030" />
          {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={styles.formGrid}>
          <div style={styles.twoColGrid}>
            <FormGroup label="Batch Number" required>
              <Input 
                name="batch_number" 
                value={formData.batch_number} 
                onChange={handleChange} 
                required 
              />
            </FormGroup>
            <FormGroup label="Quantity" required>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <Input 
                  type="number" 
                  step="0.01" 
                  name="quantity" 
                  value={formData.quantity} 
                  onChange={handleChange} 
                  required 
                  style={{ flex: 1 }} 
                />
                <Select 
                  name="unit" 
                  value={formData.unit} 
                  onChange={handleChange} 
                  style={{ width: '80px' }}
                >
                  <option value="g">g</option>
                  <option value="mg">mg</option>
                  <option value="kg">kg</option>
                  <option value="mL">mL</option>
                  <option value="L">L</option>
                  <option value="pcs">pcs</option>
                </Select>
              </div>
            </FormGroup>
          </div>
          <div style={styles.twoColGrid}>
            <FormGroup label="Supplier">
              <Input name="supplier" value={formData.supplier} onChange={handleChange} />
            </FormGroup>
            <FormGroup label="Manufacturer">
              <Input name="manufacturer" value={formData.manufacturer} onChange={handleChange} />
            </FormGroup>
          </div>
          <div style={styles.twoColGrid}>
            <FormGroup label="Received Date">
              <Input 
                type="date" 
                name="received_date" 
                value={formData.received_date} 
                onChange={handleChange} 
              />
            </FormGroup>
            <FormGroup label="Expiry Date">
              <Input 
                type="date" 
                name="expiry_date" 
                value={formData.expiry_date} 
                onChange={handleChange} 
              />
            </FormGroup>
          </div>
          <FormGroup label="Storage Location">
            <Input name="location" value={formData.location} onChange={handleChange} />
          </FormGroup>
          <FormGroup label="Notes">
            <TextArea name="notes" value={formData.notes} onChange={handleChange} rows={2} />
          </FormGroup>
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

export const CreateBatchModal = (props) => (
  <BatchFormModal {...props} title="Add Batch" batch={null} />
);

export const EditBatchModal = (props) => (
  <BatchFormModal {...props} title="Edit Batch" />
);

export default {
  CreateBatchModal,
  EditBatchModal
};
