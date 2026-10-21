// components/Modals/RequalifyBatchModal.js
// Extend a batch's expiry (retest) date after it was checked and found fit for use.
// The reason is mandatory and kept in the batch's requalification history.

import React, { useState, useEffect } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import TextArea from '../TextArea';
import Button from '../Button';
import FormGroup from '../FormGroup';
import { CheckIcon, CloseIcon, AlertCircleIcon } from '../Icons';
import { styles } from './styles';

const fmtDate = (d) => (d ? new Date(d).toLocaleDateString() : '—');

const tomorrowIso = () => {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return d.toISOString().split('T')[0];
};

export const RequalifyBatchModal = ({ isOpen, onClose, batch, onSave }) => {
  const [newDate, setNewDate] = useState('');
  const [reason, setReason] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [history, setHistory] = useState([]);

  useEffect(() => {
    if (!isOpen || !batch) return;
    setNewDate('');
    setReason('');
    setError('');
    api.getBatchRequalifications(batch.id)
      .then(rows => setHistory(Array.isArray(rows) ? rows : []))
      .catch(() => setHistory([]));
  }, [isOpen, batch]);

  if (!isOpen || !batch) return null;

  const minDate = tomorrowIso();
  const canSubmit = newDate && newDate >= minDate && reason.trim().length > 0 && !loading;

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!canSubmit) return;
    setLoading(true);
    setError('');
    try {
      await api.requalifyBatch(batch.id, {
        new_expiry_date: `${newDate}T00:00:00Z`,
        reason: reason.trim(),
      });
      onSave?.();
      onClose();
    } catch (err) {
      setError(err.message || 'Failed to extend expiry date');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`Extend expiry — batch ${batch.batch_number}`}>
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} /> {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={{ fontSize: '0.875rem', color: '#4a5568', marginBottom: '1rem' }}>
          Current expiry: <strong>{fmtDate(batch.expiry_date)}</strong>
          {' · '}status: <strong>{batch.status}</strong>
        </div>

        <FormGroup label="New expiry / retest date" required>
          <Input
            type="date"
            min={minDate}
            value={newDate}
            onChange={(e) => setNewDate(e.target.value)}
          />
        </FormGroup>

        <FormGroup
          label="Reason"
          required
          helpText="Basis for the extension, e.g. assay by titration 99.2%, appearance and mp within spec"
        >
          <TextArea
            rows={3}
            maxLength={500}
            value={reason}
            onChange={(e) => setReason(e.target.value)}
          />
        </FormGroup>

        {history.length > 0 && (
          <div style={{ marginTop: '1rem' }}>
            <div style={{ fontSize: '0.75rem', fontWeight: 700, color: '#718096', textTransform: 'uppercase', marginBottom: '6px' }}>
              Previous extensions
            </div>
            {history.map(h => (
              <div key={h.id} style={{ fontSize: '0.8rem', color: '#4a5568', padding: '6px 0', borderTop: '1px solid #edf2f7' }}>
                {fmtDate(h.created_at)} · {h.username || 'unknown'}: {fmtDate(h.old_expiry_date)} → {fmtDate(h.new_expiry_date)}
                <div style={{ color: '#718096' }}>{h.reason}</div>
              </div>
            ))}
          </div>
        )}

        <div style={styles.buttonContainer}>
          <Button variant="secondary" onClick={onClose} icon={<CloseIcon size={16} />}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" loading={loading} disabled={!canSubmit} icon={<CheckIcon size={16} />}>
            Extend
          </Button>
        </div>
      </form>
    </Modal>
  );
};

export default RequalifyBatchModal;
