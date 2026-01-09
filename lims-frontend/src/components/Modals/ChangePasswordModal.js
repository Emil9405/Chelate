// components/modals/ChangePasswordModal.js

import React, { useState, useCallback } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Button from '../Button';
import FormGroup from '../FormGroup';
import { CheckIcon, CloseIcon, AlertCircleIcon } from '../Icons';
import { styles } from './styles';
import { useFormSubmit } from './helpers';

export const ChangePasswordModal = ({ isOpen, onClose, onSave }) => {
  const [formData, setFormData] = useState({ 
    current_password: '', 
    new_password: '', 
    confirm_new_password: '' 
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const validate = useCallback(() => {
    if (formData.new_password !== formData.confirm_new_password) { 
      setError('Passwords do not match'); 
      return false; 
    }
    if (formData.new_password.length < 6) { 
      setError('Minimum 6 characters required'); 
      return false; 
    }
    setError(''); 
    return true;
  }, [formData]);

  const handleSubmit = useFormSubmit(async () => {
    setLoading(true);
    try {
      const response = await api.changePassword({ 
        current_password: formData.current_password, 
        new_password: formData.new_password 
      });
      if (response?.success) { 
        onSave(); 
        onClose(); 
      } else { 
        setError(response?.message || 'Password change error'); 
      }
    } catch (err) { 
      setError(err.message || 'Error'); 
    } finally { 
      setLoading(false); 
    }
  }, validate);

  const handleChange = (e) => {
    setFormData({ ...formData, [e.target.name]: e.target.value });
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Change Password">
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} color="#c53030" />
          {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={styles.formGrid}>
          <FormGroup label="Current Password" required>
            <Input 
              type="password" 
              name="current_password" 
              value={formData.current_password} 
              onChange={handleChange} 
              required 
            />
          </FormGroup>
          <div style={styles.twoColGrid}>
            <FormGroup label="New Password" required>
              <Input 
                type="password" 
                name="new_password" 
                value={formData.new_password} 
                onChange={handleChange} 
                required 
              />
            </FormGroup>
            <FormGroup label="Confirm New Password" required>
              <Input 
                type="password" 
                name="confirm_new_password" 
                value={formData.confirm_new_password} 
                onChange={handleChange} 
                required 
              />
            </FormGroup>
          </div>
        </div>
        <div style={styles.buttonContainer}>
          <Button variant="secondary" type="button" onClick={onClose} icon={<CloseIcon size={16} />}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" loading={loading} icon={<CheckIcon size={16} />}>
            Change Password
          </Button>
        </div>
      </form>
    </Modal>
  );
};

export default ChangePasswordModal;
