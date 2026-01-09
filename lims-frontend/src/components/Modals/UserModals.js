// components/Modals/UserModals.js

import React, { useState, useEffect } from 'react';
import { api } from '../../services/api';
import Modal from '../Modal';
import Input from '../Input';
import Select from '../Select';
import Button from '../Button';
import FormGroup from '../FormGroup';
import { CheckIcon, CloseIcon, AlertCircleIcon } from '../Icons';
import { styles } from './styles';
import { cleanPayload } from './helpers';

// ==================== CreateUserModal ====================

export const CreateUserModal = ({ isOpen, onClose, onSave }) => {
  const [formData, setFormData] = useState({
    username: '',
    email: '',
    password: '',
    confirm_password: '',
    role: 'Researcher',
    name: ''
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const validate = () => {
    if (!formData.username) {
      setError('Username is required');
      return false;
    }
    if (!formData.email) {
      setError('Email is required');
      return false;
    }
    if (!formData.password) {
      setError('Password is required');
      return false;
    }
    if (formData.password.length < 6) {
      setError('Password must be at least 6 characters');
      return false;
    }
    if (formData.password !== formData.confirm_password) {
      setError('Passwords do not match');
      return false;
    }
    setError('');
    return true;
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!validate()) return;

    setLoading(true);
    try {
      const payload = cleanPayload({
        username: formData.username,
        email: formData.email,
        password: formData.password,
        role: formData.role,
        name: formData.name || formData.username
      });

      const response = await api.createUser(payload);
      if (response && response.success !== false) {
        onSave();
        onClose();
      } else {
        setError(response?.message || 'Failed to create user');
      }
    } catch (err) {
      setError(err.message || 'Error creating user');
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (e) => {
    setFormData(prev => ({ ...prev, [e.target.name]: e.target.value }));
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Create New User">
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} color="#c53030" />
          {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={styles.formGrid}>
          <div style={styles.twoColGrid}>
            <FormGroup label="Username" required>
              <Input
                name="username"
                value={formData.username}
                onChange={handleChange}
                placeholder="johndoe"
                required
              />
            </FormGroup>
            <FormGroup label="Full Name">
              <Input
                name="name"
                value={formData.name}
                onChange={handleChange}
                placeholder="John Doe"
              />
            </FormGroup>
          </div>

          <FormGroup label="Email" required>
            <Input
              type="email"
              name="email"
              value={formData.email}
              onChange={handleChange}
              placeholder="john@example.com"
              required
            />
          </FormGroup>

          <FormGroup label="Role" required>
            <Select name="role" value={formData.role} onChange={handleChange}>
              <option value="Admin">Admin</option>
              <option value="Researcher">Researcher</option>
              <option value="Technician">Technician</option>
            </Select>
          </FormGroup>

          <div style={styles.twoColGrid}>
            <FormGroup label="Password" required>
              <Input
                type="password"
                name="password"
                value={formData.password}
                onChange={handleChange}
                placeholder="Minimum 6 characters"
                required
              />
            </FormGroup>
            <FormGroup label="Confirm Password" required>
              <Input
                type="password"
                name="confirm_password"
                value={formData.confirm_password}
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
            Create User
          </Button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== EditUserModal ====================

export const EditUserModal = ({ isOpen, onClose, user, onSave }) => {
  const [formData, setFormData] = useState({
    username: '',
    email: '',
    role: 'Researcher',
    name: '',
    is_active: true
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (user) {
      setFormData({
        username: user.username || '',
        email: user.email || '',
        role: user.role || 'Researcher',
        name: user.name || '',
        is_active: user.is_active !== false
      });
    }
  }, [user]);

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!formData.username || !formData.email) {
      setError('Username and email are required');
      return;
    }

    setLoading(true);
    try {
      const payload = cleanPayload({
        username: formData.username,
        email: formData.email,
        role: formData.role,
        name: formData.name,
        is_active: formData.is_active
      });

      const response = await api.updateUser(user.id, payload);
      if (response && response.success !== false) {
        onSave();
        onClose();
      } else {
        setError(response?.message || 'Failed to update user');
      }
    } catch (err) {
      setError(err.message || 'Error updating user');
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (e) => {
    const { name, value, type, checked } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : value
    }));
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Edit User">
      {error && (
        <div style={styles.error}>
          <AlertCircleIcon size={18} color="#c53030" />
          {error}
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <div style={styles.formGrid}>
          <div style={styles.twoColGrid}>
            <FormGroup label="Username" required>
              <Input
                name="username"
                value={formData.username}
                onChange={handleChange}
                required
              />
            </FormGroup>
            <FormGroup label="Full Name">
              <Input
                name="name"
                value={formData.name}
                onChange={handleChange}
              />
            </FormGroup>
          </div>

          <FormGroup label="Email" required>
            <Input
              type="email"
              name="email"
              value={formData.email}
              onChange={handleChange}
              required
            />
          </FormGroup>

          <div style={styles.twoColGrid}>
            <FormGroup label="Role">
              <Select name="role" value={formData.role} onChange={handleChange}>
                <option value="Admin">Admin</option>
                <option value="Researcher">Researcher</option>
                <option value="Technician">Technician</option>
              </Select>
            </FormGroup>
            <FormGroup label="Status">
              <label style={{ 
                display: 'flex', 
                alignItems: 'center', 
                gap: '8px',
                padding: '10px 0',
                cursor: 'pointer'
              }}>
                <input
                  type="checkbox"
                  name="is_active"
                  checked={formData.is_active}
                  onChange={handleChange}
                  style={{ width: '18px', height: '18px' }}
                />
                <span style={{ color: formData.is_active ? '#38a169' : '#e53e3e' }}>
                  {formData.is_active ? 'Active' : 'Inactive'}
                </span>
              </label>
            </FormGroup>
          </div>
        </div>

        <div style={styles.buttonContainer}>
          <Button variant="secondary" type="button" onClick={onClose} icon={<CloseIcon size={16} />}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" loading={loading} icon={<CheckIcon size={16} />}>
            Save Changes
          </Button>
        </div>
      </form>
    </Modal>
  );
};

// ==================== ViewUserModal ====================

export const ViewUserModal = ({ isOpen, onClose, user }) => {
  if (!isOpen || !user) return null;

  const InfoRow = ({ label, value }) => (
    <div style={{ marginBottom: '1rem' }}>
      <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>
        {label}
      </div>
      <div style={{ fontWeight: '500', color: '#1a365d' }}>
        {value || '—'}
      </div>
    </div>
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="User Details">
      <div style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr',
        gap: '1rem',
        padding: '1rem',
        backgroundColor: '#f7fafc',
        borderRadius: '8px'
      }}>
        <InfoRow label="Username" value={user.username} />
        <InfoRow label="Full Name" value={user.name} />
        <InfoRow label="Email" value={user.email} />
        <InfoRow label="Role" value={
          <span style={{
            padding: '4px 12px',
            borderRadius: '12px',
            fontSize: '0.8rem',
            fontWeight: '600',
            backgroundColor: user.role === 'Admin' ? '#fed7d7' : '#c6f6d5',
            color: user.role === 'Admin' ? '#c53030' : '#2f855a'
          }}>
            {user.role}
          </span>
        } />
        <InfoRow label="Status" value={
          <span style={{
            color: user.is_active !== false ? '#38a169' : '#e53e3e',
            fontWeight: '600'
          }}>
            {user.is_active !== false ? '● Active' : '● Inactive'}
          </span>
        } />
        <InfoRow 
          label="Created" 
          value={user.created_at ? new Date(user.created_at).toLocaleDateString('ru-RU') : '—'} 
        />
      </div>

      <div style={styles.buttonContainer}>
        <Button variant="secondary" onClick={onClose} icon={<CloseIcon size={16} />}>
          Close
        </Button>
      </div>
    </Modal>
  );
};

export default {
  CreateUserModal,
  EditUserModal,
  ViewUserModal
};
