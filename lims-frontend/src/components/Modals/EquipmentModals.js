// components/modals/EquipmentModals.js
// Equipment modals with new design matching screenshot

import React, { useState, useEffect, useRef } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';

// ==================== LOCAL STYLES ====================

const styles = {
  container: {
    display: 'flex',
    gap: '1.5rem',
    marginBottom: '1.5rem'
  },
  requiredSection: {
    flex: 1,
    borderLeft: '3px solid #3b82f6',
    paddingLeft: '1rem'
  },
  sectionTitle: {
    fontSize: '0.75rem',
    fontWeight: '600',
    color: '#6b7280',
    marginBottom: '1rem',
    textTransform: 'uppercase',
    letterSpacing: '0.05em'
  },
  photoSection: {
    width: '140px',
    flexShrink: 0
  },
  photoBox: {
    border: '1px solid #e5e7eb',
    borderRadius: '8px',
    padding: '1rem',
    textAlign: 'center',
    backgroundColor: '#f9fafb'
  },
  photoPlaceholder: {
    width: '60px',
    height: '60px',
    margin: '0 auto 0.5rem',
    backgroundColor: '#e5e7eb',
    borderRadius: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: '#9ca3af'
  },
  photoLabel: {
    fontSize: '0.75rem',
    color: '#9ca3af',
    marginBottom: '0.5rem'
  },
  additionalSection: {
    marginBottom: '1rem'
  },
  row: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '1rem',
    marginBottom: '0.75rem'
  },
  fullRow: {
    marginBottom: '0.75rem'
  },
  label: {
    display: 'block',
    fontSize: '0.75rem',
    fontWeight: '500',
    color: '#374151',
    marginBottom: '0.25rem'
  },
  required: {
    color: '#ef4444'
  },
  input: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    fontSize: '0.875rem',
    color: '#1f2937',
    backgroundColor: '#fff',
    outline: 'none',
    boxSizing: 'border-box',
    transition: 'border-color 0.2s'
  },
  select: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    fontSize: '0.875rem',
    color: '#1f2937',
    backgroundColor: '#fff',
    cursor: 'pointer',
    outline: 'none',
    boxSizing: 'border-box'
  },
  textarea: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    fontSize: '0.875rem',
    color: '#1f2937',
    backgroundColor: '#fff',
    outline: 'none',
    boxSizing: 'border-box',
    minHeight: '80px',
    resize: 'vertical'
  },
  buttonContainer: {
    display: 'flex',
    gap: '0.75rem',
    justifyContent: 'flex-end',
    paddingTop: '1rem',
    borderTop: '1px solid #e5e7eb'
  },
  cancelBtn: {
    padding: '0.5rem 1rem',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    backgroundColor: '#fff',
    color: '#374151',
    fontSize: '0.875rem',
    cursor: 'pointer',
    fontWeight: '500'
  },
  submitBtn: {
    padding: '0.5rem 1rem',
    border: 'none',
    borderRadius: '6px',
    backgroundColor: '#3b82f6',
    color: '#fff',
    fontSize: '0.875rem',
    cursor: 'pointer',
    fontWeight: '500'
  },
  submitBtnDisabled: {
    backgroundColor: '#93c5fd',
    cursor: 'not-allowed'
  },
  error: {
    color: '#dc2626',
    backgroundColor: '#fef2f2',
    padding: '0.75rem',
    borderRadius: '6px',
    marginBottom: '1rem',
    fontSize: '0.875rem',
    border: '1px solid #fecaca'
  },
  previewImage: {
    width: '60px',
    height: '60px',
    objectFit: 'cover',
    borderRadius: '8px',
    margin: '0 auto 0.5rem',
    display: 'block'
  },
  fileInput: {
    display: 'none'
  },
  browseBtn: {
    padding: '0.25rem 0.75rem',
    border: '1px solid #d1d5db',
    borderRadius: '4px',
    backgroundColor: '#fff',
    color: '#374151',
    fontSize: '0.75rem',
    cursor: 'pointer'
  }
};

// ==================== FORM FIELD COMPONENT ====================

const FormField = ({ label, required, children }) => (
  <div>
    <label style={styles.label}>
      {label}
      {required && <span style={styles.required}>*</span>}
    </label>
    {children}
  </div>
);

// ==================== IMAGE PREVIEW COMPONENT ====================

const ImageUpload = ({ file, existingUrl, onFileSelect }) => {
  const [preview, setPreview] = useState(null);
  const fileInputRef = useRef(null);

  useEffect(() => {
    if (file) {
      const reader = new FileReader();
      reader.onloadend = () => setPreview(reader.result);
      reader.readAsDataURL(file);
    } else {
      setPreview(null);
    }
  }, [file]);

  const imageUrl = preview || existingUrl;

  const handleClick = () => {
    fileInputRef.current?.click();
  };

  const handleChange = (e) => {
    if (e.target.files && e.target.files[0]) {
      onFileSelect(e.target.files[0]);
    }
  };

  return (
    <div style={styles.photoSection}>
      <div style={styles.sectionTitle}>Photo</div>
      <div style={styles.photoBox}>
        {imageUrl ? (
          <img src={imageUrl} alt="Preview" style={styles.previewImage} />
        ) : (
          <div style={styles.photoPlaceholder}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
              <circle cx="8.5" cy="8.5" r="1.5"/>
              <polyline points="21 15 16 10 5 21"/>
            </svg>
          </div>
        )}
        <div style={styles.photoLabel}>{imageUrl ? 'Change photo' : 'Add photo'}</div>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          onChange={handleChange}
          style={styles.fileInput}
        />
        <button type="button" onClick={handleClick} style={styles.browseBtn}>
          Browse...
        </button>
      </div>
    </div>
  );
};

// ==================== EQUIPMENT FORM MODAL ====================

const EquipmentFormModal = ({ isOpen, onClose, title, equipment = null, existingImage = null, onSave }) => {
  const isEdit = !!equipment;
  
  const [formData, setFormData] = useState({
    name: '',
    type_: 'instrument',
    quantity: 1,
    unit: 'pcs',
    status: 'available',
    location: '',
    description: '',
    serial_number: '',
    manufacturer: '',
    model: '',
    purchase_date: '',
    warranty_until: '',
    maintenance_interval_days: 90
  });

  const [imageFile, setImageFile] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (equipment) {
      setFormData({
        name: equipment.name || '',
        type_: equipment.type_ || 'instrument',
        quantity: equipment.quantity || 1,
        unit: equipment.unit || 'pcs',
        status: equipment.status || 'available',
        location: equipment.location || '',
        description: equipment.description || '',
        serial_number: equipment.serial_number || '',
        manufacturer: equipment.manufacturer || '',
        model: equipment.model || '',
        purchase_date: equipment.purchase_date?.split('T')[0] || '',
        warranty_until: equipment.warranty_until?.split('T')[0] || '',
        maintenance_interval_days: equipment.maintenance_interval_days || 90
      });
    } else {
      // Reset form for new equipment
      setFormData({
        name: '',
        type_: 'instrument',
        quantity: 1,
        unit: 'pcs',
        status: 'available',
        location: '',
        description: '',
        serial_number: '',
        manufacturer: '',
        model: '',
        purchase_date: '',
        warranty_until: '',
        maintenance_interval_days: 90
      });
    }
    setImageFile(null);
    setError('');
  }, [equipment, isOpen]);

  const cleanPayload = (data) => {
    const payload = { ...data };
    Object.keys(payload).forEach(key => {
      if (payload[key] === '' || payload[key] === null) {
        delete payload[key];
      }
    });
    return payload;
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!formData.name.trim()) {
      setError('Name is required');
      return;
    }

    setLoading(true);
    setError('');
    
    try {
      const payload = cleanPayload(formData);
      if (payload.maintenance_interval_days) {
        payload.maintenance_interval_days = parseInt(payload.maintenance_interval_days);
      }
      if (payload.quantity) {
        payload.quantity = parseInt(payload.quantity);
      }
      
      let response;
      if (isEdit) {
        response = await api.updateEquipment(equipment.id, payload);
      } else {
        response = await api.createEquipment(payload);
      }
      
      if (response && response.success !== false) { 
        const equipmentId = isEdit ? equipment.id : (response.data?.id || response.id);

        // Upload image if selected
        if (imageFile && equipmentId) {
          try {
            await api.uploadEquipmentFile(equipmentId, imageFile, { file_type: 'image' });
          } catch (uploadErr) {
            console.error("Failed to upload image:", uploadErr);
          }
        }

        onSave(); 
        onClose(); 
      } else { 
        setError(response?.message || 'Error saving equipment'); 
      }
    } catch (err) { 
      setError(err.message); 
    } finally { 
      setLoading(false); 
    }
  };

  const handleChange = (e) => {
    const { name, value } = e.target;
    setFormData(prev => ({ ...prev, [name]: value }));
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title}>
      {error && <div style={styles.error}>{error}</div>}
      
      <form onSubmit={handleSubmit}>
        {/* Top section: Required + Photo */}
        <div style={styles.container}>
          {/* Required Section */}
          <div style={styles.requiredSection}>
            <div style={styles.sectionTitle}>Required</div>
            
            <div style={styles.fullRow}>
              <FormField label="Name" required>
                <input
                  type="text"
                  name="name"
                  value={formData.name}
                  onChange={handleChange}
                  style={styles.input}
                  placeholder=""
                />
              </FormField>
            </div>
            
            <div style={styles.row}>
              <FormField label="Type" required>
                <select name="type_" value={formData.type_} onChange={handleChange} style={styles.select}>
                  <option value="instrument">Instrument</option>
                  <option value="glassware">Glassware</option>
                  <option value="safety">Safety</option>
                  <option value="storage">Storage</option>
                  <option value="consumable">Consumable</option>
                  <option value="other">Other</option>
                </select>
              </FormField>
              <FormField label="Quantity" required>
                <input
                  type="number"
                  name="quantity"
                  value={formData.quantity}
                  onChange={handleChange}
                  min="1"
                  style={styles.input}
                />
              </FormField>
            </div>
          </div>

          {/* Photo Section */}
          <ImageUpload
            file={imageFile}
            existingUrl={isEdit ? existingImage : null}
            onFileSelect={setImageFile}
          />
        </div>

        {/* Additional Section */}
        <div style={styles.additionalSection}>
          <div style={styles.sectionTitle}>Additional</div>
          
          <div style={styles.row}>
            <FormField label="Unit">
              <input
                type="text"
                name="unit"
                value={formData.unit}
                onChange={handleChange}
                style={styles.input}
                placeholder="pcs"
              />
            </FormField>
            <FormField label="Model">
              <input
                type="text"
                name="model"
                value={formData.model}
                onChange={handleChange}
                style={styles.input}
              />
            </FormField>
          </div>

          <div style={styles.row}>
            <FormField label="Serial Number">
              <input
                type="text"
                name="serial_number"
                value={formData.serial_number}
                onChange={handleChange}
                style={styles.input}
              />
            </FormField>
            <FormField label="Manufacturer">
              <input
                type="text"
                name="manufacturer"
                value={formData.manufacturer}
                onChange={handleChange}
                style={styles.input}
              />
            </FormField>
          </div>

          <div style={styles.row}>
            <FormField label="Purchase Date">
              <input
                type="date"
                name="purchase_date"
                value={formData.purchase_date}
                onChange={handleChange}
                style={styles.input}
              />
            </FormField>
            <FormField label="Warranty Until">
              <input
                type="date"
                name="warranty_until"
                value={formData.warranty_until}
                onChange={handleChange}
                style={styles.input}
              />
            </FormField>
          </div>

          <div style={styles.fullRow}>
            <FormField label="Location">
              <input
                type="text"
                name="location"
                value={formData.location}
                onChange={handleChange}
                style={styles.input}
              />
            </FormField>
          </div>

          <div style={styles.fullRow}>
            <FormField label="Description">
              <textarea
                name="description"
                value={formData.description}
                onChange={handleChange}
                style={styles.textarea}
                rows={3}
              />
            </FormField>
          </div>
        </div>

        {/* Buttons */}
        <div style={styles.buttonContainer}>
          <button type="button" onClick={onClose} style={styles.cancelBtn}>
            Cancel
          </button>
          <button 
            type="submit" 
            style={{
              ...styles.submitBtn,
              ...(loading ? styles.submitBtnDisabled : {})
            }}
            disabled={loading}
          >
            {loading ? 'Saving...' : (isEdit ? 'Save Changes' : 'Create Equipment')}
          </button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== EXPORTS ====================

export const CreateEquipmentModal = (props) => (
  <EquipmentFormModal {...props} title="Add Equipment" equipment={null} />
);

export const EditEquipmentModal = (props) => (
  <EquipmentFormModal {...props} title="Edit Equipment" />
);

export default {
  CreateEquipmentModal,
  EditEquipmentModal
};
