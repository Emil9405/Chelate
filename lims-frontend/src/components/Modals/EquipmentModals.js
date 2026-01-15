// components/modals/EquipmentModals.js
// All equipment-related modals: Create/Edit Equipment, Part, Maintenance, Details

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Badge from '../Badge';
import Button from '../Button';
import Loading from '../Loading';
import ErrorMessage from '../ErrorMessage';

// ==================== CONSTANTS ====================

// IMPORTANT: Database constraint requires EXACTLY: 'manual', 'certificate', 'photo', 'other'
// DO NOT use 'image', 'specification', 'maintenance_log' - they will fail!
const FILE_TYPES = { 
  photo: 'Photo',
  manual: 'Manual', 
  certificate: 'Certificate', 
  other: 'Other' 
};

const PART_STATUSES = {
  good: { label: 'Good', color: 'success' },
  needs_attention: { label: 'Needs Attention', color: 'warning' },
  needs_replacement: { label: 'Needs Replacement', color: 'danger' },
  replaced: { label: 'Replaced', color: 'info' },
  missing: { label: 'Missing', color: 'secondary' }
};

const EQUIPMENT_STATUSES = {
  available: { label: 'Available', color: 'success' },
  in_use: { label: 'In Use', color: 'info' },
  maintenance: { label: 'Maintenance', color: 'warning' },
  damaged: { label: 'Damaged', color: 'danger' },
  calibration: { label: 'Calibration', color: 'secondary' },
  retired: { label: 'Retired', color: 'secondary' }
};

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8080';
const getFileUrl = (equipmentId, fileId) => `${API_BASE_URL}/api/v1/public/equipment/${equipmentId}/files/${fileId}`;

// ==================== STYLES ====================

const styles = {
  container: { display: 'flex', gap: '1.5rem', marginBottom: '1.5rem' },
  requiredSection: { flex: 1, borderLeft: '3px solid #3b82f6', paddingLeft: '1rem' },
  sectionTitle: { fontSize: '0.75rem', fontWeight: '600', color: '#6b7280', marginBottom: '1rem', textTransform: 'uppercase', letterSpacing: '0.05em' },
  photoSection: { width: '140px', flexShrink: 0 },
  photoBox: { border: '1px solid #e5e7eb', borderRadius: '8px', padding: '1rem', textAlign: 'center', backgroundColor: '#f9fafb' },
  photoPlaceholder: { width: '60px', height: '60px', margin: '0 auto 0.5rem', backgroundColor: '#e5e7eb', borderRadius: '8px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#9ca3af' },
  photoLabel: { fontSize: '0.75rem', color: '#9ca3af', marginBottom: '0.5rem' },
  row: { display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '0.75rem' },
  row3: { display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '1rem', marginBottom: '0.75rem' },
  fullRow: { marginBottom: '0.75rem' },
  label: { display: 'block', fontSize: '0.75rem', fontWeight: '500', color: '#374151', marginBottom: '0.25rem' },
  required: { color: '#ef4444' },
  input: { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #d1d5db', borderRadius: '6px', fontSize: '0.875rem', color: '#1f2937', backgroundColor: '#fff', outline: 'none', boxSizing: 'border-box' },
  select: { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #d1d5db', borderRadius: '6px', fontSize: '0.875rem', color: '#1f2937', backgroundColor: '#fff', cursor: 'pointer', outline: 'none', boxSizing: 'border-box' },
  textarea: { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #d1d5db', borderRadius: '6px', fontSize: '0.875rem', color: '#1f2937', backgroundColor: '#fff', outline: 'none', boxSizing: 'border-box', minHeight: '80px', resize: 'vertical' },
  buttonContainer: { display: 'flex', gap: '0.75rem', justifyContent: 'flex-end', paddingTop: '1rem', borderTop: '1px solid #e5e7eb' },
  cancelBtn: { padding: '0.5rem 1rem', border: '1px solid #d1d5db', borderRadius: '6px', backgroundColor: '#fff', color: '#374151', fontSize: '0.875rem', cursor: 'pointer', fontWeight: '500' },
  submitBtn: { padding: '0.5rem 1rem', border: 'none', borderRadius: '6px', backgroundColor: '#3b82f6', color: '#fff', fontSize: '0.875rem', cursor: 'pointer', fontWeight: '500' },
  submitBtnDisabled: { backgroundColor: '#93c5fd', cursor: 'not-allowed' },
  error: { color: '#dc2626', backgroundColor: '#fef2f2', padding: '0.75rem', borderRadius: '6px', marginBottom: '1rem', fontSize: '0.875rem', border: '1px solid #fecaca' },
  previewImage: { width: '60px', height: '60px', objectFit: 'cover', borderRadius: '8px', margin: '0 auto 0.5rem', display: 'block' },
  fileInput: { display: 'none' },
  browseBtn: { padding: '0.25rem 0.75rem', border: '1px solid #d1d5db', borderRadius: '4px', backgroundColor: '#fff', color: '#374151', fontSize: '0.75rem', cursor: 'pointer' }
};

const inputStyle = styles.input;

// ==================== FORM COMPONENTS ====================

const FormField = ({ label, required, children }) => (
  <div>
    <label style={styles.label}>
      {label}
      {required && <span style={styles.required}>*</span>}
    </label>
    {children}
  </div>
);

const FormGroup = ({ label, required, children, style }) => (
  <div style={{ marginBottom: '1rem', ...style }}>
    {label && (
      <label style={{ display: 'block', fontWeight: '500', marginBottom: '0.5rem', color: '#374151', fontSize: '0.875rem' }}>
        {label}{required && <span style={{ color: '#ef4444', marginLeft: '2px' }}>*</span>}
      </label>
    )}
    {children}
  </div>
);

const FormInput = ({ label, required, type = 'text', ...props }) => (
  <FormGroup label={label} required={required}>
    <input type={type} style={inputStyle} {...props} />
  </FormGroup>
);

const FormSelect = ({ label, required, children, ...props }) => (
  <FormGroup label={label} required={required}>
    <select style={{ ...inputStyle, cursor: 'pointer' }} {...props}>{children}</select>
  </FormGroup>
);

const FormTextarea = ({ label, required, ...props }) => (
  <FormGroup label={label} required={required}>
    <textarea style={{ ...inputStyle, minHeight: '80px', resize: 'vertical' }} {...props} />
  </FormGroup>
);

// ==================== IMAGE PREVIEW (for forms) ====================

const ImagePreview = ({ file, onRemove }) => {
  const [preview, setPreview] = useState(null);
  
  useEffect(() => {
    if (file) {
      const reader = new FileReader();
      reader.onloadend = () => setPreview(reader.result);
      reader.readAsDataURL(file);
    } else {
      setPreview(null);
    }
  }, [file]);

  if (!preview) return null;
  
  return (
    <div style={{ position: 'relative', display: 'inline-block', marginTop: '0.5rem' }}>
      <img src={preview} alt="Preview" style={{ width: '100px', height: '100px', objectFit: 'cover', borderRadius: '8px', border: '2px solid #e2e8f0' }} />
      <button 
        type="button" 
        onClick={onRemove} 
        style={{ position: 'absolute', top: '-8px', right: '-8px', background: '#ef4444', color: 'white', border: 'none', borderRadius: '50%', width: '24px', height: '24px', cursor: 'pointer', fontSize: '0.75rem' }}
      >
        <i className="fas fa-times"></i>
      </button>
    </div>
  );
};

// ==================== HOVER IMAGE (zoom on hover) ====================

const HoverImage = ({ src, alt, size = 40, zoomSize = 200 }) => {
  const [error, setError] = useState(false);
  const [showZoom, setShowZoom] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });

  const handleMouseEnter = (e) => {
    if (!src || error) return;
    const rect = e.currentTarget.getBoundingClientRect();
    setPosition({ x: rect.right + 10, y: rect.top });
    setShowZoom(true);
  };

  if (error || !src) {
    return (
      <div style={{ width: size, height: size, borderRadius: '6px', background: '#e2e8f0', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#718096', flexShrink: 0 }}>
        <i className="fas fa-image" style={{ fontSize: size * 0.4 }}></i>
      </div>
    );
  }

  return (
    <>
      <img
        src={src}
        alt={alt}
        style={{ width: size, height: size, objectFit: 'cover', borderRadius: '6px', flexShrink: 0, cursor: 'pointer', transition: 'transform 0.2s' }}
        onError={() => setError(true)}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={() => setShowZoom(false)}
      />
      {showZoom && (
        <div style={{
          position: 'fixed',
          left: Math.min(position.x, window.innerWidth - zoomSize - 20),
          top: Math.max(10, Math.min(position.y, window.innerHeight - zoomSize - 20)),
          zIndex: 9999,
          pointerEvents: 'none'
        }}>
          <img src={src} alt={alt} style={{ width: zoomSize, height: zoomSize, objectFit: 'cover', borderRadius: '12px', border: '3px solid white', boxShadow: '0 10px 40px rgba(0,0,0,0.3)' }} />
        </div>
      )}
    </>
  );
};

// ==================== IMAGE UPLOAD COMPONENT ====================

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
          onChange={(e) => e.target.files?.[0] && onFileSelect(e.target.files[0])}
          style={styles.fileInput}
        />
        <button type="button" onClick={() => fileInputRef.current?.click()} style={styles.browseBtn}>
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
    name: '', type_: 'instrument', quantity: 1, unit: 'pcs', status: 'available',
    location: '', description: '', serial_number: '', manufacturer: '', model: '',
    purchase_date: '', warranty_until: '', maintenance_interval_days: 90
  });
  const [imageFile, setImageFile] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (equipment) {
      setFormData({
        name: equipment.name || '', type_: equipment.type_ || 'instrument',
        quantity: equipment.quantity || 1, unit: equipment.unit || 'pcs',
        status: equipment.status || 'available', location: equipment.location || '',
        description: equipment.description || '', serial_number: equipment.serial_number || '',
        manufacturer: equipment.manufacturer || '', model: equipment.model || '',
        purchase_date: equipment.purchase_date?.split('T')[0] || '',
        warranty_until: equipment.warranty_until?.split('T')[0] || '',
        maintenance_interval_days: equipment.maintenance_interval_days || 90
      });
    } else {
      setFormData({
        name: '', type_: 'instrument', quantity: 1, unit: 'pcs', status: 'available',
        location: '', description: '', serial_number: '', manufacturer: '', model: '',
        purchase_date: '', warranty_until: '', maintenance_interval_days: 90
      });
    }
    setImageFile(null);
    setError('');
  }, [equipment, isOpen]);

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!formData.name.trim()) { setError('Name is required'); return; }

    setLoading(true);
    setError('');
    
    try {
      const payload = { ...formData };
      Object.keys(payload).forEach(key => {
        if (payload[key] === '' || payload[key] === null) delete payload[key];
      });
      if (payload.maintenance_interval_days) payload.maintenance_interval_days = parseInt(payload.maintenance_interval_days);
      if (payload.quantity) payload.quantity = parseInt(payload.quantity);
      
      let response;
      if (isEdit) {
        response = await api.updateEquipment(equipment.id, payload);
      } else {
        response = await api.createEquipment(payload);
      }
      
      if (response && response.success !== false) { 
        const equipmentId = isEdit ? equipment.id : (response.data?.id || response.id);
        
        // Upload image - use 'photo' not 'image'!
        if (imageFile && equipmentId) {
          try {
            await api.uploadEquipmentFile(equipmentId, imageFile, { file_type: 'photo' });
          } catch (uploadErr) {
            console.error("Failed to upload photo:", uploadErr);
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
        <div style={styles.container}>
          <div style={styles.requiredSection}>
            <div style={styles.sectionTitle}>Required</div>
            <div style={styles.fullRow}>
              <FormField label="Name" required>
                <input type="text" name="name" value={formData.name} onChange={handleChange} style={styles.input} />
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
                <input type="number" name="quantity" value={formData.quantity} onChange={handleChange} min="1" style={styles.input} />
              </FormField>
            </div>
          </div>
          <ImageUpload file={imageFile} existingUrl={isEdit ? existingImage : null} onFileSelect={setImageFile} />
        </div>

        <div style={{ marginBottom: '1rem' }}>
          <div style={styles.sectionTitle}>Additional</div>
          <div style={styles.row}>
            <FormField label="Unit"><input type="text" name="unit" value={formData.unit} onChange={handleChange} style={styles.input} placeholder="pcs" /></FormField>
            <FormField label="Model"><input type="text" name="model" value={formData.model} onChange={handleChange} style={styles.input} /></FormField>
          </div>
          <div style={styles.row}>
            <FormField label="Serial Number"><input type="text" name="serial_number" value={formData.serial_number} onChange={handleChange} style={styles.input} /></FormField>
            <FormField label="Manufacturer"><input type="text" name="manufacturer" value={formData.manufacturer} onChange={handleChange} style={styles.input} /></FormField>
          </div>
          <div style={styles.row}>
            <FormField label="Purchase Date"><input type="date" name="purchase_date" value={formData.purchase_date} onChange={handleChange} style={styles.input} /></FormField>
            <FormField label="Warranty Until"><input type="date" name="warranty_until" value={formData.warranty_until} onChange={handleChange} style={styles.input} /></FormField>
          </div>
          <div style={styles.fullRow}>
            <FormField label="Location"><input type="text" name="location" value={formData.location} onChange={handleChange} style={styles.input} /></FormField>
          </div>
          <div style={styles.fullRow}>
            <FormField label="Description"><textarea name="description" value={formData.description} onChange={handleChange} style={styles.textarea} rows={3} /></FormField>
          </div>
        </div>

        <div style={styles.buttonContainer}>
          <button type="button" onClick={onClose} style={styles.cancelBtn}>Cancel</button>
          <button type="submit" style={{ ...styles.submitBtn, ...(loading ? styles.submitBtnDisabled : {}) }} disabled={loading}>
            {loading ? 'Saving...' : (isEdit ? 'Save Changes' : 'Create Equipment')}
          </button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== PART FORM MODAL ====================

export const PartFormModal = ({ equipmentId, part, existingImageUrl, onClose, onSave }) => {
  const [formData, setFormData] = useState(part || { 
    name: '', part_number: '', manufacturer: '', quantity: 1, min_quantity: 0, status: 'good', notes: '' 
  });
  const [photo, setPhoto] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!formData.name?.trim()) { setError('Part name is required'); return; }
    
    try {
      setLoading(true); 
      setError('');
      const payload = { 
        name: formData.name.trim(), 
        quantity: parseInt(formData.quantity) || 1, 
        min_quantity: parseInt(formData.min_quantity) || 0, 
        status: formData.status || 'good' 
      };
      ['part_number', 'manufacturer', 'notes'].forEach(k => { 
        if (formData[k]?.trim()) payload[k] = formData[k].trim(); 
      });
      
      if (part) { 
        await api.updateEquipmentPart(equipmentId, part.id, payload);
        if (photo) {
          try { 
            await api.uploadEquipmentFile(equipmentId, photo, { file_type: 'photo', description: `Part: ${formData.name.trim()}`, part_id: part.id }); 
          } catch (e) { console.error(e); }
        }
      } else {
        const created = await api.createEquipmentPart(equipmentId, payload);
        if (photo && created?.id) {
          try { 
            await api.uploadEquipmentFile(equipmentId, photo, { file_type: 'photo', description: `Part: ${formData.name.trim()}`, part_id: created.id }); 
          } catch (e) { console.error(e); }
        }
      }
      onSave();
    } catch (err) { 
      setError(err.message); 
    } finally { 
      setLoading(false); 
    }
  };

  return (
    <div style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1001, padding: '2rem' }}>
      <div style={{ background: 'white', borderRadius: '12px', width: '100%', maxWidth: '700px', maxHeight: '90vh', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h2 style={{ margin: 0, fontSize: '1.25rem', fontWeight: '600', color: '#2d3748' }}>{part ? 'Edit Part' : 'Add Part'}</h2>
          <button onClick={onClose} style={{ background: 'none', border: 'none', fontSize: '1.25rem', cursor: 'pointer', color: '#718096' }}><i className="fas fa-times"></i></button>
        </div>
        <div style={{ flex: 1, overflow: 'auto', padding: '1.5rem 2rem' }}>
          {error && <div style={styles.error}>{error}</div>}
          <form onSubmit={handleSubmit}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 150px', gap: '1.5rem' }}>
              <div>
                <FormInput label="Name" required value={formData.name} onChange={(e) => setFormData({...formData, name: e.target.value})} />
                <div style={styles.row}>
                  <FormInput label="Part Number" value={formData.part_number} onChange={(e) => setFormData({...formData, part_number: e.target.value})} />
                  <FormInput label="Manufacturer" value={formData.manufacturer} onChange={(e) => setFormData({...formData, manufacturer: e.target.value})} />
                </div>
                <div style={styles.row3}>
                  <FormInput label="Qty" type="number" min="0" value={formData.quantity} onChange={(e) => setFormData({...formData, quantity: parseInt(e.target.value) || 0})} />
                  <FormInput label="Min" type="number" min="0" value={formData.min_quantity} onChange={(e) => setFormData({...formData, min_quantity: parseInt(e.target.value) || 0})} />
                  <FormSelect label="Status" value={formData.status} onChange={(e) => setFormData({...formData, status: e.target.value})}>
                    {Object.entries(PART_STATUSES).map(([v, { label }]) => <option key={v} value={v}>{label}</option>)}
                  </FormSelect>
                </div>
                <FormTextarea label="Notes" value={formData.notes} onChange={(e) => setFormData({...formData, notes: e.target.value})} />
              </div>
              <div>
                <div style={{ padding: '0.75rem', background: '#f8fafc', borderRadius: '8px', border: '1px solid #e2e8f0' }}>
                  <h4 style={{ margin: '0 0 0.75rem 0', color: '#374151', fontSize: '0.75rem', fontWeight: '600' }}>Photo</h4>
                  {existingImageUrl && !photo && (
                    <div style={{ marginBottom: '0.5rem' }}>
                      <img src={existingImageUrl} alt="Current" style={{ width: '100%', maxWidth: '120px', borderRadius: '6px' }} />
                      <p style={{ fontSize: '0.7rem', color: '#718096', margin: '0.25rem 0 0 0' }}>Current</p>
                    </div>
                  )}
                  <div style={{ border: '2px dashed #d1d5db', borderRadius: '8px', padding: '0.75rem', textAlign: 'center', background: photo ? '#f0fdf4' : 'white' }}>
                    {photo ? (
                      <ImagePreview file={photo} onRemove={() => setPhoto(null)} />
                    ) : (
                      <>
                        <i className="fas fa-camera" style={{ fontSize: '1.5rem', color: '#9ca3af' }}></i>
                        <p style={{ margin: '0.25rem 0 0 0', fontSize: '0.7rem', color: '#6b7280' }}>{existingImageUrl ? 'Replace' : 'Add'}</p>
                      </>
                    )}
                    <input type="file" accept="image/*" onChange={(e) => setPhoto(e.target.files[0])} style={{ display: photo ? 'none' : 'block', width: '100%', marginTop: '0.5rem', fontSize: '0.7rem' }} />
                  </div>
                </div>
              </div>
            </div>
            <div style={styles.buttonContainer}>
              <Button variant="secondary" type="button" onClick={onClose}>Cancel</Button>
              <Button variant="primary" type="submit" disabled={loading}>{loading ? 'Saving...' : 'Save'}</Button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
};

// ==================== MAINTENANCE FORM MODAL ====================

export const MaintenanceFormModal = ({ equipmentId, onClose, onSave }) => {
  const [formData, setFormData] = useState({ 
    maintenance_type: 'scheduled', 
    scheduled_date: new Date().toISOString().split('T')[0], 
    description: '', 
    cost: '' 
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e) => {
    e.preventDefault();
    try {
      setLoading(true); 
      setError('');
      const payload = { maintenance_type: formData.maintenance_type, scheduled_date: formData.scheduled_date };
      if (formData.description?.trim()) payload.description = formData.description.trim();
      if (formData.cost && parseFloat(formData.cost) > 0) payload.cost = parseFloat(formData.cost);
      await api.createMaintenance(equipmentId, payload);
      onSave();
    } catch (err) { 
      setError(err.message); 
    } finally { 
      setLoading(false); 
    }
  };

  return (
    <Modal isOpen={true} onClose={onClose} title="Schedule Maintenance">
      {error && <div style={styles.error}>{error}</div>}
      <form onSubmit={handleSubmit}>
        <div style={styles.row}>
          <FormSelect label="Type" value={formData.maintenance_type} onChange={(e) => setFormData({...formData, maintenance_type: e.target.value})}>
            <option value="scheduled">Scheduled</option>
            <option value="calibration">Calibration</option>
            <option value="repair">Repair</option>
            <option value="inspection">Inspection</option>
            <option value="cleaning">Cleaning</option>
            <option value="part_replacement">Part Replacement</option>
          </FormSelect>
          <FormInput label="Date" required type="date" value={formData.scheduled_date} onChange={(e) => setFormData({...formData, scheduled_date: e.target.value})} />
        </div>
        <FormInput label="Cost" type="number" step="0.01" min="0" value={formData.cost} onChange={(e) => setFormData({...formData, cost: e.target.value})} />
        <FormTextarea label="Description" value={formData.description} onChange={(e) => setFormData({...formData, description: e.target.value})} />
        <div style={{ display: 'flex', gap: '1rem', justifyContent: 'flex-end', marginTop: '1.5rem' }}>
          <Button variant="secondary" type="button" onClick={onClose}>Cancel</Button>
          <Button variant="primary" type="submit" disabled={loading}>{loading ? 'Scheduling...' : 'Schedule'}</Button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== EQUIPMENT DETAILS MODAL ====================

export const EquipmentDetailsModal = ({ equipment, user, onClose, onUpdate }) => {
  const [activeTab, setActiveTab] = useState('details');
  const [parts, setParts] = useState([]);
  const [maintenance, setMaintenance] = useState([]);
  const [files, setFiles] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const canEdit = useCallback(() => ['Admin', 'Researcher'].includes(user?.role), [user?.role]);

  const loadAll = useCallback(async () => {
    try {
      setLoading(true);
      const [p, m, f] = await Promise.all([
        api.getEquipmentParts(equipment.id),
        api.getEquipmentMaintenance(equipment.id),
        api.getEquipmentFiles(equipment.id)
      ]);
      setParts(Array.isArray(p) ? p : []);
      setMaintenance(Array.isArray(m) ? m : []);
      setFiles(Array.isArray(f) ? f : []);
    } catch (err) { 
      setError(err.message); 
    } finally { 
      setLoading(false); 
    }
  }, [equipment.id]);

  useEffect(() => { loadAll(); }, [loadAll]);

  const handleRefresh = () => { loadAll(); onUpdate(); };

  const tabs = [
    { id: 'details', label: 'Details', icon: 'fas fa-info-circle' },
    { id: 'parts', label: 'Parts', icon: 'fas fa-cogs' },
    { id: 'maintenance', label: 'Maintenance', icon: 'fas fa-wrench' },
    { id: 'files', label: 'Files', icon: 'fas fa-file-alt' },
  ];

  const mainImage = files.find(f => f.file_type === 'photo' && !f.part_id);

  return (
    <div style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000, padding: '2rem' }}>
      <div style={{ background: 'white', borderRadius: '12px', width: '100%', maxWidth: '1000px', maxHeight: '90vh', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h2 style={{ margin: 0, fontSize: '1.5rem', fontWeight: '600', color: '#2d3748' }}>{equipment.name}</h2>
          <button onClick={onClose} style={{ background: 'none', border: 'none', fontSize: '1.5rem', cursor: 'pointer', color: '#718096' }}><i className="fas fa-times"></i></button>
        </div>

        <div style={{ flex: 1, overflow: 'auto', padding: '1.5rem 2rem' }}>
          {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}
          
          {/* Header with Photo */}
          <div style={{ display: 'flex', gap: '1.5rem', marginBottom: '1.5rem', padding: '1rem', background: '#f8fafc', borderRadius: '8px' }}>
            <div style={{ flexShrink: 0 }}>
              {mainImage ? (
                <HoverImage src={getFileUrl(equipment.id, mainImage.id)} alt={equipment.name} size={120} zoomSize={300} />
              ) : (
                <div style={{ width: '120px', height: '120px', display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#e2e8f0', borderRadius: '8px', color: '#718096' }}>
                  <i className="fas fa-tools" style={{ fontSize: '2.5rem' }}></i>
                </div>
              )}
            </div>
            <div style={{ flex: 1 }}>
              <h3 style={{ margin: '0 0 0.5rem 0', color: '#2d3748' }}>{equipment.name}</h3>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem', fontSize: '0.875rem', color: '#718096' }}>
                {equipment.type_ && <span><i className="fas fa-tag"></i> {equipment.type_}</span>}
                {equipment.manufacturer && <span><i className="fas fa-industry"></i> {equipment.manufacturer}</span>}
                {equipment.serial_number && <span><i className="fas fa-barcode"></i> {equipment.serial_number}</span>}
                {equipment.location && <span><i className="fas fa-map-marker-alt"></i> {equipment.location}</span>}
              </div>
              <div style={{ marginTop: '0.75rem' }}>
                <Badge variant={EQUIPMENT_STATUSES[equipment.status]?.color || 'secondary'}>{EQUIPMENT_STATUSES[equipment.status]?.label || equipment.status}</Badge>
              </div>
            </div>
          </div>

          {/* Tabs */}
          <div style={{ display: 'flex', borderBottom: '2px solid #e2e8f0', marginBottom: '1.5rem' }}>
            {tabs.map(tab => (
              <button key={tab.id} onClick={() => setActiveTab(tab.id)} style={{
                padding: '0.75rem 1.5rem', border: 'none', background: 'none', cursor: 'pointer',
                color: activeTab === tab.id ? '#667eea' : '#718096',
                borderBottom: activeTab === tab.id ? '2px solid #667eea' : '2px solid transparent',
                marginBottom: '-2px', fontWeight: activeTab === tab.id ? '600' : '400',
              }}>
                <i className={tab.icon} style={{ marginRight: '0.5rem' }}></i>{tab.label}
              </button>
            ))}
          </div>

          {loading && <Loading />}

          {activeTab === 'details' && !loading && <DetailsTab equipment={equipment} />}
          {activeTab === 'parts' && !loading && <PartsTab equipmentId={equipment.id} parts={parts} files={files} canEdit={canEdit()} onRefresh={handleRefresh} />}
          {activeTab === 'maintenance' && !loading && <MaintenanceTab equipmentId={equipment.id} maintenance={maintenance} canEdit={canEdit()} onRefresh={handleRefresh} />}
          {activeTab === 'files' && !loading && <FilesTab equipmentId={equipment.id} files={files} canEdit={canEdit()} onRefresh={handleRefresh} />}
        </div>
      </div>
    </div>
  );
};

// ==================== TAB COMPONENTS ====================

const DetailsTab = ({ equipment }) => {
  const fields = [
    { key: 'name', label: 'Name' },
    { key: 'type_', label: 'Type' },
    { key: 'status', label: 'Status', render: (v) => EQUIPMENT_STATUSES[v]?.label || v },
    { key: 'quantity', label: 'Quantity' },
    { key: 'unit', label: 'Unit' },
    { key: 'serial_number', label: 'Serial Number' },
    { key: 'manufacturer', label: 'Manufacturer' },
    { key: 'model', label: 'Model' },
    { key: 'location', label: 'Location' },
    { key: 'purchase_date', label: 'Purchase Date', render: (v) => v ? new Date(v).toLocaleDateString() : '-' },
    { key: 'warranty_until', label: 'Warranty Until', render: (v) => v ? new Date(v).toLocaleDateString() : '-' },
    { key: 'description', label: 'Description' },
  ];

  return (
    <table style={{ width: '100%', borderCollapse: 'collapse' }}>
      <tbody>
        {fields.map(field => (
          <tr key={field.key} style={{ borderBottom: '1px solid #e2e8f0' }}>
            <td style={{ padding: '0.75rem 1rem', width: '200px', background: '#f8fafc', fontWeight: '500', color: '#4a5568', fontSize: '0.875rem' }}>{field.label}</td>
            <td style={{ padding: '0.75rem 1rem', color: equipment[field.key] ? '#2d3748' : '#a0aec0' }}>
              {field.render ? field.render(equipment[field.key]) : (equipment[field.key] || '-')}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
};

const PartsTab = ({ equipmentId, parts, files, canEdit, onRefresh }) => {
  const [showAddModal, setShowAddModal] = useState(false);
  const [editingPart, setEditingPart] = useState(null);
  const [searchTerm, setSearchTerm] = useState('');

  const handleDelete = async (partId) => {
    if (window.confirm('Delete this part?')) {
      await api.deleteEquipmentPart(equipmentId, partId);
      onRefresh();
    }
  };

  const getPartImage = (partId) => {
    const img = files.find(f => f.file_type === 'photo' && f.part_id === partId);
    return img ? getFileUrl(equipmentId, img.id) : null;
  };

  const filteredParts = parts.filter(part => {
    if (!searchTerm) return true;
    const term = searchTerm.toLowerCase();
    return part.name?.toLowerCase().includes(term) || part.part_number?.toLowerCase().includes(term);
  });

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem', gap: '1rem' }}>
        <div style={{ flex: 1, maxWidth: '300px' }}>
          <input placeholder="Search parts..." value={searchTerm} onChange={(e) => setSearchTerm(e.target.value)} style={inputStyle} />
        </div>
        {canEdit && <Button variant="primary" onClick={() => setShowAddModal(true)}><i className="fas fa-plus"></i> Add Part</Button>}
      </div>

      {filteredParts.length === 0 ? (
        <p style={{ textAlign: 'center', color: '#718096', padding: '2rem' }}>{parts.length === 0 ? 'No parts registered' : 'No parts match search'}</p>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ background: '#f8fafc', borderBottom: '2px solid #e2e8f0' }}>
              <th style={{ width: '60px', padding: '0.75rem 1rem' }}></th>
              <th style={{ textAlign: 'left', padding: '0.75rem 1rem', color: '#4a5568', fontWeight: '600' }}>Name</th>
              <th style={{ textAlign: 'left', padding: '0.75rem 1rem', color: '#4a5568', fontWeight: '600' }}>Part Number</th>
              <th style={{ textAlign: 'left', padding: '0.75rem 1rem', color: '#4a5568', fontWeight: '600' }}>Manufacturer</th>
              <th style={{ textAlign: 'center', padding: '0.75rem 1rem', color: '#4a5568', fontWeight: '600' }}>Qty</th>
              <th style={{ textAlign: 'left', padding: '0.75rem 1rem', color: '#4a5568', fontWeight: '600' }}>Status</th>
              {canEdit && <th style={{ textAlign: 'right', padding: '0.75rem 1rem' }}>Actions</th>}
            </tr>
          </thead>
          <tbody>
            {filteredParts.map(part => (
              <tr key={part.id} style={{ borderBottom: '1px solid #e2e8f0' }}>
                <td style={{ padding: '0.5rem 1rem' }}>
                  <HoverImage src={getPartImage(part.id)} alt={part.name} size={40} zoomSize={200} />
                </td>
                <td style={{ padding: '0.75rem 1rem', fontWeight: '500', color: '#2d3748' }}>{part.name}</td>
                <td style={{ padding: '0.75rem 1rem', color: '#718096', fontFamily: 'monospace' }}>{part.part_number || '-'}</td>
                <td style={{ padding: '0.75rem 1rem', color: '#718096' }}>{part.manufacturer || '-'}</td>
                <td style={{ padding: '0.75rem 1rem', textAlign: 'center' }}>{part.quantity}</td>
                <td style={{ padding: '0.75rem 1rem' }}>
                  <Badge variant={PART_STATUSES[part.status]?.color || 'secondary'}>{PART_STATUSES[part.status]?.label || part.status}</Badge>
                </td>
                {canEdit && (
                  <td style={{ padding: '0.75rem 1rem', textAlign: 'right' }}>
                    <Button variant="secondary" size="small" onClick={() => setEditingPart(part)} style={{ marginRight: '0.5rem' }}><i className="fas fa-edit"></i></Button>
                    <Button variant="danger" size="small" onClick={() => handleDelete(part.id)}><i className="fas fa-trash"></i></Button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {showAddModal && <PartFormModal equipmentId={equipmentId} onClose={() => setShowAddModal(false)} onSave={() => { setShowAddModal(false); onRefresh(); }} />}
      {editingPart && <PartFormModal equipmentId={equipmentId} part={editingPart} existingImageUrl={getPartImage(editingPart.id)} onClose={() => setEditingPart(null)} onSave={() => { setEditingPart(null); onRefresh(); }} />}
    </div>
  );
};

const MaintenanceTab = ({ equipmentId, maintenance, canEdit, onRefresh }) => {
  const [showAddModal, setShowAddModal] = useState(false);
  
  const handleDelete = async (id) => { 
    if (window.confirm('Delete?')) { 
      await api.deleteMaintenance(equipmentId, id); 
      onRefresh(); 
    } 
  };
  
  const handleComplete = async (id) => { 
    const by = prompt('Performed by:'); 
    if (by) { 
      await api.completeMaintenance(equipmentId, id, { performed_by: by }); 
      onRefresh(); 
    } 
  };
  
  const statusColors = { scheduled: 'info', in_progress: 'warning', completed: 'success', cancelled: 'secondary', overdue: 'danger' };

  return (
    <div>
      {canEdit && <div style={{ marginBottom: '1rem' }}><Button variant="primary" onClick={() => setShowAddModal(true)}><i className="fas fa-plus"></i> Schedule</Button></div>}
      {maintenance.length === 0 ? <p style={{ textAlign: 'center', color: '#718096', padding: '2rem' }}>No maintenance records</p> : (
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ borderBottom: '2px solid #e2e8f0' }}>
              <th style={{ textAlign: 'left', padding: '0.75rem' }}>Type</th>
              <th style={{ textAlign: 'left', padding: '0.75rem' }}>Scheduled</th>
              <th style={{ textAlign: 'left', padding: '0.75rem' }}>Status</th>
              <th style={{ textAlign: 'left', padding: '0.75rem' }}>Performed By</th>
              {canEdit && <th style={{ textAlign: 'right', padding: '0.75rem' }}>Actions</th>}
            </tr>
          </thead>
          <tbody>
            {maintenance.map(m => (
              <tr key={m.id} style={{ borderBottom: '1px solid #e2e8f0' }}>
                <td style={{ padding: '0.75rem' }}>{m.maintenance_type?.replace('_', ' ').toUpperCase()}</td>
                <td style={{ padding: '0.75rem' }}>{m.scheduled_date ? new Date(m.scheduled_date).toLocaleDateString() : '-'}</td>
                <td style={{ padding: '0.75rem' }}><Badge variant={statusColors[m.status] || 'secondary'}>{m.status?.toUpperCase()}</Badge></td>
                <td style={{ padding: '0.75rem', color: '#718096' }}>{m.performed_by || '-'}</td>
                {canEdit && (
                  <td style={{ padding: '0.75rem', textAlign: 'right' }}>
                    {m.status === 'scheduled' && <Button variant="success" size="small" onClick={() => handleComplete(m.id)} style={{ marginRight: '0.5rem' }}><i className="fas fa-check"></i></Button>}
                    <Button variant="danger" size="small" onClick={() => handleDelete(m.id)}><i className="fas fa-trash"></i></Button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {showAddModal && <MaintenanceFormModal equipmentId={equipmentId} onClose={() => setShowAddModal(false)} onSave={() => { setShowAddModal(false); onRefresh(); }} />}
    </div>
  );
};

const FilesTab = ({ equipmentId, files, canEdit, onRefresh }) => {
  const [uploading, setUploading] = useState(false);
  const [uploadForm, setUploadForm] = useState({ file: null, file_type: 'photo', description: '' });
  const [uploadError, setUploadError] = useState('');
  const fileTypeIcons = { manual: 'fa-book', photo: 'fa-image', certificate: 'fa-certificate', other: 'fa-file' };

  const handleUpload = async (e) => {
    e.preventDefault();
    if (!uploadForm.file) return;
    try {
      setUploading(true); 
      setUploadError('');
      await api.uploadEquipmentFile(equipmentId, uploadForm.file, { file_type: uploadForm.file_type, description: uploadForm.description });
      setUploadForm({ file: null, file_type: 'photo', description: '' });
      document.getElementById('equipment-file-input').value = '';
      onRefresh();
    } catch (err) { 
      setUploadError(err.message); 
    } finally { 
      setUploading(false); 
    }
  };

  const handleDownload = async (file) => { 
    try { 
      await api.downloadEquipmentFile(equipmentId, file.id, file.original_filename); 
    } catch (err) { 
      alert(err.message); 
    } 
  };
  
  const handleDelete = async (fileId) => { 
    if (window.confirm('Delete?')) { 
      await api.deleteEquipmentFile(equipmentId, fileId); 
      onRefresh(); 
    } 
  };

  const equipmentImages = files.filter(f => f.file_type === 'photo' && !f.part_id);
  const partImages = files.filter(f => f.file_type === 'photo' && f.part_id);
  const documents = files.filter(f => f.file_type !== 'photo');

  return (
    <div>
      {canEdit && (
        <form onSubmit={handleUpload} style={{ marginBottom: '1.5rem', padding: '1rem', background: '#f7fafc', borderRadius: '8px' }}>
          {uploadError && <div style={styles.error}>{uploadError}</div>}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem', alignItems: 'end' }}>
            <FormGroup label="File"><input id="equipment-file-input" type="file" onChange={(e) => setUploadForm({ ...uploadForm, file: e.target.files[0] })} /></FormGroup>
            <FormSelect label="Type" value={uploadForm.file_type} onChange={(e) => setUploadForm({ ...uploadForm, file_type: e.target.value })}>
              {Object.entries(FILE_TYPES).map(([v, l]) => <option key={v} value={v}>{l}</option>)}
            </FormSelect>
            <FormInput label="Description" value={uploadForm.description} onChange={(e) => setUploadForm({ ...uploadForm, description: e.target.value })} placeholder="Optional" />
            <div style={{ marginBottom: '1rem' }}><Button variant="primary" type="submit" disabled={!uploadForm.file || uploading}>{uploading ? 'Uploading...' : 'Upload'}</Button></div>
          </div>
        </form>
      )}

      {equipmentImages.length > 0 && (
        <div style={{ marginBottom: '1.5rem' }}>
          <h4 style={{ margin: '0 0 1rem 0', color: '#4a5568', fontSize: '0.875rem' }}><i className="fas fa-images"></i> Equipment Photos ({equipmentImages.length})</h4>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem' }}>
            {equipmentImages.map(file => (
              <div key={file.id} style={{ position: 'relative' }}>
                <HoverImage src={getFileUrl(equipmentId, file.id)} alt={file.original_filename} size={120} zoomSize={300} />
                {canEdit && <button onClick={() => handleDelete(file.id)} style={{ position: 'absolute', top: '4px', right: '4px', background: 'rgba(220,38,38,0.9)', color: 'white', border: 'none', borderRadius: '50%', width: '24px', height: '24px', cursor: 'pointer' }}><i className="fas fa-times"></i></button>}
              </div>
            ))}
          </div>
        </div>
      )}

      {partImages.length > 0 && (
        <div style={{ marginBottom: '1.5rem' }}>
          <h4 style={{ margin: '0 0 1rem 0', color: '#4a5568', fontSize: '0.875rem' }}><i className="fas fa-cogs"></i> Part Photos ({partImages.length})</h4>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem' }}>
            {partImages.map(file => (
              <div key={file.id} style={{ position: 'relative' }}>
                <HoverImage src={getFileUrl(equipmentId, file.id)} alt={file.original_filename} size={80} zoomSize={200} />
                {file.description && <div style={{ fontSize: '0.7rem', color: '#718096', textAlign: 'center', marginTop: '0.25rem', maxWidth: '80px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{file.description}</div>}
                {canEdit && <button onClick={() => handleDelete(file.id)} style={{ position: 'absolute', top: '2px', right: '2px', background: 'rgba(220,38,38,0.9)', color: 'white', border: 'none', borderRadius: '50%', width: '20px', height: '20px', cursor: 'pointer', fontSize: '0.6rem' }}><i className="fas fa-times"></i></button>}
              </div>
            ))}
          </div>
        </div>
      )}

      {documents.length > 0 && (
        <div>
          <h4 style={{ margin: '0 0 1rem 0', color: '#4a5568', fontSize: '0.875rem' }}><i className="fas fa-folder"></i> Documents ({documents.length})</h4>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '1rem' }}>
            {documents.map(file => (
              <div key={file.id} style={{ padding: '1rem', border: '1px solid #e2e8f0', borderRadius: '8px' }}>
                <div style={{ display: 'flex', alignItems: 'center', marginBottom: '0.5rem' }}>
                  <i className={`fas ${fileTypeIcons[file.file_type] || 'fa-file'}`} style={{ fontSize: '1.5rem', marginRight: '0.75rem', color: '#667eea' }}></i>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <p style={{ margin: 0, fontWeight: '500', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{file.original_filename}</p>
                    <p style={{ margin: 0, fontSize: '0.75rem', color: '#718096' }}>{FILE_TYPES[file.file_type]} • {(file.file_size / 1024).toFixed(1)} KB</p>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <Button variant="secondary" size="small" onClick={() => handleDownload(file)}><i className="fas fa-download"></i></Button>
                  {canEdit && <Button variant="danger" size="small" onClick={() => handleDelete(file.id)}><i className="fas fa-trash"></i></Button>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {files.length === 0 && <p style={{ textAlign: 'center', color: '#718096', padding: '2rem' }}>No files uploaded</p>}
    </div>
  );
};

// ==================== EXPORTS ====================

export const CreateEquipmentModal = (props) => (
  <EquipmentFormModal {...props} title="Add Equipment" equipment={null} />
);

export const EditEquipmentModal = (props) => (
  <EquipmentFormModal {...props} title="Edit Equipment" />
);

// Export HoverImage for use in Equipment.js
export { HoverImage, getFileUrl, PART_STATUSES, EQUIPMENT_STATUSES, FILE_TYPES };

export default {
  CreateEquipmentModal,
  EditEquipmentModal,
  PartFormModal,
  MaintenanceFormModal,
  EquipmentDetailsModal,
  HoverImage
};