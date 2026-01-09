// components/modals/UsageHistoryModal.js

import React, { useState, useEffect } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Button from '../Button';
import FormGroup from '../FormGroup';
import Table from '../Table';
import { SaveIcon, CloseIcon } from '../Icons';
import { styles } from './styles';

export const UsageHistoryModal = ({ isOpen, onClose, reagentId, batchId, onSave }) => {
  const [history, setHistory] = useState([]);
  const [usageForm, setUsageForm] = useState({ 
    quantity_used: '', 
    purpose: '', 
    notes: '' 
  });
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (isOpen && batchId) {
      loadHistory();
    }
    // eslint-disable-next-line
  }, [isOpen, batchId, reagentId]);

  const loadHistory = async () => {
    try {
      const res = await api.getUsageHistory(reagentId, batchId);
      const data = Array.isArray(res) ? res : (res.data?.data || res.data || []);
      setHistory(Array.isArray(data) ? data : []);
    } catch (err) {
      console.error('Failed to load usage history:', err);
      setHistory([]);
    }
  };

  const handleAdd = async (e) => {
    e.preventDefault();
    if (!usageForm.quantity_used) return;
    
    setLoading(true);
    try {
      await api.useReagent(reagentId, batchId, { 
        ...usageForm, 
        quantity_used: parseFloat(usageForm.quantity_used) 
      });
      
      if (onSave) onSave();
      await loadHistory();
      setUsageForm({ quantity_used: '', purpose: '', notes: '' });
    } catch (err) { 
      console.error('Failed to record usage:', err); 
    } finally { 
      setLoading(false); 
    }
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Reagent Usage">
      {/* Usage Form */}
      <form 
        onSubmit={handleAdd} 
        style={{ 
          background: '#f7fafc', 
          padding: '1rem', 
          borderRadius: '8px', 
          marginBottom: '1rem' 
        }}
      >
        <div style={{ 
          display: 'grid', 
          gridTemplateColumns: '100px 1fr 1fr auto', 
          gap: '0.75rem', 
          alignItems: 'end' 
        }}>
          <FormGroup label="Quantity" required>
            <Input 
              type="number" 
              step="0.01" 
              value={usageForm.quantity_used} 
              onChange={e => setUsageForm({ ...usageForm, quantity_used: e.target.value })} 
              required 
            />
          </FormGroup>
          <FormGroup label="Purpose">
            <Input 
              value={usageForm.purpose} 
              onChange={e => setUsageForm({ ...usageForm, purpose: e.target.value })} 
              placeholder="Experiment, analysis..." 
            />
          </FormGroup>
          <FormGroup label="Notes">
            <Input 
              value={usageForm.notes} 
              onChange={e => setUsageForm({ ...usageForm, notes: e.target.value })} 
            />
          </FormGroup>
          <Button type="submit" variant="primary" loading={loading} icon={<SaveIcon size={16} />}>
            Save
          </Button>
        </div>
      </form>

      {/* History Table */}
      <Table
        data={history}
        columns={[
          { 
            key: 'used_at', 
            label: 'Date', 
            render: i => new Date(i.used_at).toLocaleDateString('ru') 
          },
          { key: 'quantity_used', label: 'Quantity' },
          { key: 'purpose', label: 'Purpose' },
          { key: 'username', label: 'User', render: i => i.username || '—' }
        ]}
        emptyMessage="History is empty"
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
