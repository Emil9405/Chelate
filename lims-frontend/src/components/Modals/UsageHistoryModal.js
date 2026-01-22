// components/modals/UsageHistoryModal.js
// v2.0: Added unit-based dispensing support

import React, { useState, useEffect } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Button from '../Button';
import FormGroup from '../FormGroup';
import Table from '../Table';
import { SaveIcon, CloseIcon, PackageIcon } from '../Icons';
import { styles } from './styles';

// Иконка для штучного списания (если PackageIcon нет)
const BoxIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2">
    <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
    <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
    <line x1="12" y1="22.08" x2="12" y2="12"/>
  </svg>
);

export const UsageHistoryModal = ({ isOpen, onClose, reagentId, batchId, batch, onSave }) => {
  // Support both direct props and batch object
  const actualReagentId = reagentId || batch?.reagent_id;
  const actualBatchId = batchId || batch?.id;
  const batchUnit = batch?.unit || '';
  const batchNumber = batch?.batch_number || '';
  const availableQuantity = batch?.quantity || 0;
  const packSize = batch?.pack_size;

  const [history, setHistory] = useState([]);
  const [unitsInfo, setUnitsInfo] = useState(null);
  const [usageMode, setUsageMode] = useState('quantity'); // 'quantity' | 'units'
  
  // Form for quantity-based usage
  const [usageForm, setUsageForm] = useState({ 
    quantity_used: '', 
    purpose: '', 
    notes: '' 
  });
  
  // Form for unit-based dispensing
  const [unitsForm, setUnitsForm] = useState({
    units_to_dispense: 1,
    purpose: '',
    notes: ''
  });
  
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [successMessage, setSuccessMessage] = useState('');

  useEffect(() => {
    if (isOpen && actualBatchId && actualReagentId) {
      loadHistory();
      loadUnitsInfo();
    }
    // eslint-disable-next-line
  }, [isOpen, actualBatchId, actualReagentId]);

  const loadHistory = async () => {
    try {
      setError('');
      const res = await api.getUsageHistory(actualReagentId, actualBatchId);
      const data = Array.isArray(res) ? res : (res.data?.data || res.data || []);
      setHistory(Array.isArray(data) ? data : []);
    } catch (err) {
      console.error('Failed to load usage history:', err);
      setError(err.message || 'Failed to load history');
      setHistory([]);
    }
  };

  const loadUnitsInfo = async () => {
    try {
      const info = await api.getBatchUnitsInfo(actualReagentId, actualBatchId);
      setUnitsInfo(info);
      // Автоматически переключаемся на штучный режим если pack_size установлен
      if (info?.can_dispense_by_units && info?.pack_size) {
        setUsageMode('units');
      }
    } catch (err) {
      console.error('Failed to load units info:', err);
      // Не критичная ошибка - просто не показываем штучный режим
      setUnitsInfo(null);
    }
  };

  // Обычное списание по количеству
  const handleQuantityUsage = async (e) => {
    e.preventDefault();
    if (!usageForm.quantity_used) return;
    
    const quantity = parseFloat(usageForm.quantity_used);
    if (quantity > availableQuantity) {
      setError(`Cannot use more than available (${availableQuantity} ${batchUnit})`);
      return;
    }
    
    setLoading(true);
    setError('');
    setSuccessMessage('');
    
    try {
      await api.useReagent(actualReagentId, actualBatchId, { 
        ...usageForm, 
        quantity_used: quantity 
      });
      
      setSuccessMessage(`Used ${quantity} ${batchUnit}`);
      if (onSave) onSave();
      await loadHistory();
      await loadUnitsInfo();
      setUsageForm({ quantity_used: '', purpose: '', notes: '' });
    } catch (err) { 
      console.error('Failed to record usage:', err);
      setError(err.message || 'Failed to record usage');
    } finally { 
      setLoading(false); 
    }
  };

  // Штучное списание
  const handleUnitsDispense = async (e) => {
    e.preventDefault();
    if (!unitsForm.units_to_dispense || unitsForm.units_to_dispense < 1) return;
    
    const units = parseInt(unitsForm.units_to_dispense, 10);
    if (unitsInfo && units > unitsInfo.available_units) {
      setError(`Cannot dispense more than available (${unitsInfo.available_units} units)`);
      return;
    }
    
    setLoading(true);
    setError('');
    setSuccessMessage('');
    
    try {
      const result = await api.dispenseUnits(actualReagentId, actualBatchId, {
        units_to_dispense: units,
        purpose: unitsForm.purpose || null,
        notes: unitsForm.notes || null
      });
      
      setSuccessMessage(`Dispensed ${result.units_dispensed} unit(s) (${result.quantity_dispensed} ${result.unit})`);
      if (onSave) onSave();
      await loadHistory();
      await loadUnitsInfo();
      setUnitsForm({ units_to_dispense: 1, purpose: '', notes: '' });
    } catch (err) {
      console.error('Failed to dispense units:', err);
      setError(err.message || 'Failed to dispense units');
    } finally {
      setLoading(false);
    }
  };

  const canUseUnitMode = unitsInfo?.can_dispense_by_units && unitsInfo?.pack_size > 0;

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`Reagent Usage${batchNumber ? ` - Batch ${batchNumber}` : ''}`}>
      {/* Batch Info */}
      {batch && (
        <div style={{ 
          background: '#edf2f7', 
          padding: '0.75rem 1rem', 
          borderRadius: '8px', 
          marginBottom: '1rem',
          display: 'flex',
          flexWrap: 'wrap',
          gap: '1.5rem',
          fontSize: '0.9rem'
        }}>
          <div><strong>Available:</strong> {unitsInfo?.available_quantity ?? availableQuantity} {batchUnit}</div>
          {canUseUnitMode && (
            <div><strong>Units:</strong> {unitsInfo.available_units} × {unitsInfo.pack_size} {batchUnit}</div>
          )}
          <div><strong>Status:</strong> {batch.status}</div>
          {batch.location && <div><strong>Location:</strong> {batch.location}</div>}
        </div>
      )}

      {/* Success Message */}
      {successMessage && (
        <div style={{ 
          background: '#c6f6d5', 
          color: '#276749', 
          padding: '0.75rem 1rem', 
          borderRadius: '8px', 
          marginBottom: '1rem',
          display: 'flex',
          alignItems: 'center',
          gap: '0.5rem'
        }}>
          ✓ {successMessage}
        </div>
      )}

      {/* Error Message */}
      {error && (
        <div style={{ 
          background: '#fed7d7', 
          color: '#c53030', 
          padding: '0.75rem 1rem', 
          borderRadius: '8px', 
          marginBottom: '1rem' 
        }}>
          {error}
        </div>
      )}

      {/* Mode Toggle (only if unit mode available) */}
      {canUseUnitMode && (
        <div style={{ 
          display: 'flex', 
          gap: '0.5rem', 
          marginBottom: '1rem',
          background: '#f7fafc',
          padding: '0.5rem',
          borderRadius: '8px'
        }}>
          <button
            type="button"
            onClick={() => setUsageMode('quantity')}
            style={{
              flex: 1,
              padding: '0.5rem 1rem',
              border: 'none',
              borderRadius: '6px',
              cursor: 'pointer',
              fontWeight: usageMode === 'quantity' ? '600' : '400',
              background: usageMode === 'quantity' ? '#3182ce' : 'transparent',
              color: usageMode === 'quantity' ? 'white' : '#4a5568',
              transition: 'all 0.2s'
            }}
          >
            By Quantity ({batchUnit})
          </button>
          <button
            type="button"
            onClick={() => setUsageMode('units')}
            style={{
              flex: 1,
              padding: '0.5rem 1rem',
              border: 'none',
              borderRadius: '6px',
              cursor: 'pointer',
              fontWeight: usageMode === 'units' ? '600' : '400',
              background: usageMode === 'units' ? '#3182ce' : 'transparent',
              color: usageMode === 'units' ? 'white' : '#4a5568',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem',
              transition: 'all 0.2s'
            }}
          >
            <BoxIcon size={16} /> By Units ({unitsInfo?.pack_size} {batchUnit}/unit)
          </button>
        </div>
      )}

      {/* Usage Form - Quantity Mode */}
      {usageMode === 'quantity' && (
        <form 
          onSubmit={handleQuantityUsage} 
          style={{ 
            background: '#f7fafc', 
            padding: '1rem', 
            borderRadius: '8px', 
            marginBottom: '1rem' 
          }}
        >
          <div style={{ 
            display: 'grid', 
            gridTemplateColumns: '140px 1fr 1fr auto', 
            gap: '0.75rem', 
            alignItems: 'flex-end' 
          }}>
            <FormGroup label={`Quantity (${batchUnit})`} required style={{ marginBottom: 0 }}>
              <Input 
                type="number" 
                step="0.01"
                min="0.01"
                max={unitsInfo?.available_quantity ?? availableQuantity}
                value={usageForm.quantity_used} 
                onChange={e => setUsageForm({ ...usageForm, quantity_used: e.target.value })} 
                required 
                placeholder={`Max: ${unitsInfo?.available_quantity ?? availableQuantity}`}
              />
            </FormGroup>
            <FormGroup label="Purpose" style={{ marginBottom: 0 }}>
              <Input 
                value={usageForm.purpose} 
                onChange={e => setUsageForm({ ...usageForm, purpose: e.target.value })} 
                placeholder="Experiment, analysis..." 
              />
            </FormGroup>
            <FormGroup label="Notes" style={{ marginBottom: 0 }}>
              <Input 
                value={usageForm.notes} 
                onChange={e => setUsageForm({ ...usageForm, notes: e.target.value })} 
              />
            </FormGroup>
            <Button 
              type="submit" 
              variant="primary" 
              loading={loading} 
              icon={<SaveIcon size={16} />}
              style={{ height: '38px' }}
            >
              Use
            </Button>
          </div>
        </form>
      )}

      {/* Usage Form - Units Mode */}
      {usageMode === 'units' && canUseUnitMode && (
        <form 
          onSubmit={handleUnitsDispense} 
          style={{ 
            background: '#ebf8ff', 
            padding: '1rem', 
            borderRadius: '8px', 
            marginBottom: '1rem',
            border: '1px solid #90cdf4'
          }}
        >
          <div style={{ 
            display: 'grid', 
            gridTemplateColumns: '140px 1fr 1fr auto', 
            gap: '0.75rem', 
            alignItems: 'flex-end' 
          }}>
            <FormGroup label="Units to dispense" required style={{ marginBottom: 0 }}>
              <Input 
                type="number" 
                step="1"
                min="1"
                max={unitsInfo?.available_units || 999}
                value={unitsForm.units_to_dispense} 
                onChange={e => setUnitsForm({ ...unitsForm, units_to_dispense: e.target.value })} 
                required 
              />
            </FormGroup>
            <FormGroup label="Purpose" style={{ marginBottom: 0 }}>
              <Input 
                value={unitsForm.purpose} 
                onChange={e => setUnitsForm({ ...unitsForm, purpose: e.target.value })} 
                placeholder="Experiment, analysis..." 
              />
            </FormGroup>
            <FormGroup label="Notes" style={{ marginBottom: 0 }}>
              <Input 
                value={unitsForm.notes} 
                onChange={e => setUnitsForm({ ...unitsForm, notes: e.target.value })} 
              />
            </FormGroup>
            <Button 
              type="submit" 
              variant="primary" 
              loading={loading} 
              icon={<BoxIcon size={16} />}
              style={{ height: '38px' }}
            >
              Dispense
            </Button>
          </div>
          
          {/* Preview */}
          <div style={{ 
            marginTop: '0.75rem', 
            padding: '0.5rem 0.75rem', 
            background: 'white', 
            borderRadius: '4px',
            fontSize: '0.85rem',
            color: '#4a5568'
          }}>
            Will dispense: <strong>{unitsForm.units_to_dispense || 0} × {unitsInfo.pack_size} = {(unitsForm.units_to_dispense || 0) * unitsInfo.pack_size} {batchUnit}</strong>
            <span style={{ marginLeft: '1rem', color: '#718096' }}>
              (Remaining: {unitsInfo.available_units - (unitsForm.units_to_dispense || 0)} units)
            </span>
          </div>
        </form>
      )}

      {/* History Table */}
      <Table
        data={history}
        columns={[
          { 
            key: 'used_at', 
            label: 'Date', 
            render: i => new Date(i.used_at || i.created_at).toLocaleDateString('ru-RU', { 
              day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' 
            })
          },
          { 
            key: 'quantity_used', 
            label: 'Quantity',
            render: i => `${i.quantity_used} ${i.unit || batchUnit}`
          },
          { key: 'purpose', label: 'Purpose', render: i => i.purpose || '—' },
          { key: 'username', label: 'User', render: i => i.username || '—' }
        ]}
        emptyMessage="No usage history yet"
      />

      <div style={styles.buttonContainer}>
        <Button variant="secondary" onClick={onClose} icon={<CloseIcon size={16} />}>
          Close
        </Button>
      </div>
    </Modal>
  );
};

export default UsageHistoryModal;