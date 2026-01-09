// components/Users.js - Updated with proper hooks and UserModals
import React, { useState, useEffect, useCallback } from 'react';
import { api } from '../services/api';
import ErrorMessage from './ErrorMessage';
import Loading from './Loading';
import Table from './Table';
import Badge from './Badge';
import Button from './Button';
import Input from './Input';
import Select from './Select';
import { CreateUserModal, EditUserModal, ViewUserModal } from './Modals';

const Users = ({ user }) => {
  const [users, setUsers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [roleFilter, setRoleFilter] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showViewModal, setShowViewModal] = useState(false);
  const [selectedUser, setSelectedUser] = useState(null);
  const [error, setError] = useState('');

  const isAdmin = user?.role?.toLowerCase() === 'admin';

  // Мемоизированная функция загрузки
  const loadUsers = useCallback(async () => {
    if (!isAdmin) return;
    
    try {
      setError('');
      setLoading(true);
      const data = await api.getUsers();
      
      // Нормализация ответа API
      let userList = [];
      if (Array.isArray(data)) {
        userList = data;
      } else if (data?.users && Array.isArray(data.users)) {
        userList = data.users;
      } else if (data?.data && Array.isArray(data.data)) {
        userList = data.data;
      } else {
        console.warn('Unexpected data format from API:', data);
        setError('Received unexpected data format from server');
      }
      
      setUsers(userList);
    } catch (err) {
      console.error('Failed to load users:', err);
      setError(err.message || 'Failed to load users');
      setUsers([]);
    } finally {
      setLoading(false);
    }
  }, [isAdmin]);

  // Загрузка при монтировании и изменении роли
  useEffect(() => {
    if (isAdmin) {
      loadUsers();
    } else {
      setLoading(false);
    }
  }, [isAdmin, loadUsers]);

  // Обработчик действий над пользователями
  const handleAction = useCallback(async (action, item) => {
    try {
      switch (action) {
        case 'view':
          setSelectedUser(item);
          setShowViewModal(true);
          break;
          
        case 'edit':
          setSelectedUser(item);
          setShowEditModal(true);
          break;
          
        case 'delete':
          if (window.confirm(`Are you sure you want to delete user "${item.username || item.email}"?`)) {
            await api.deleteUser(item.id);
            loadUsers();
          }
          break;
          
        case 'reset':
          const newPassword = prompt('Enter new password for user:');
          if (newPassword) {
            if (newPassword.length < 6) {
              setError('Password must be at least 6 characters');
              return;
            }
            await api.resetUserPassword(item.id, newPassword);
            alert('Password reset successfully');
          }
          break;
          
        default:
          console.warn('Unknown action:', action);
      }
    } catch (err) {
      console.error(`Failed to ${action} user:`, err);
      setError(err.message || `Failed to ${action} user`);
    }
  }, [loadUsers]);

  // Фильтрация пользователей
  const filteredUsers = users.filter(u => {
    if (!u) return false;
    
    const searchLower = searchTerm.toLowerCase();
    const matchesSearch = !searchTerm || 
      u.email?.toLowerCase().includes(searchLower) ||
      u.name?.toLowerCase().includes(searchLower) ||
      u.username?.toLowerCase().includes(searchLower);
    
    const matchesRole = !roleFilter || u.role === roleFilter;
    
    return matchesSearch && matchesRole;
  });

  // Обработчики модалов
  const handleCreateSuccess = useCallback(() => {
    setShowCreateModal(false);
    loadUsers();
  }, [loadUsers]);

  const handleEditSuccess = useCallback(() => {
    setShowEditModal(false);
    setSelectedUser(null);
    loadUsers();
  }, [loadUsers]);

  const handleCloseEdit = useCallback(() => {
    setShowEditModal(false);
    setSelectedUser(null);
  }, []);

  const handleCloseView = useCallback(() => {
    setShowViewModal(false);
    setSelectedUser(null);
  }, []);

  // Проверка доступа — ПОСЛЕ всех хуков
  if (!isAdmin) {
    return (
      <div style={{ padding: '6rem 2rem 2rem 2rem' }}>
        <div style={{
          background: 'white',
          borderRadius: '12px',
          boxShadow: '0 4px 20px rgba(0, 0, 0, 0.05)',
          padding: '2rem',
          textAlign: 'center'
        }}>
          <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>🔒</div>
          <h2 style={{ color: '#e53e3e', marginBottom: '0.5rem' }}>Access Denied</h2>
          <p style={{ color: '#718096' }}>Admin privileges required to view this page.</p>
        </div>
      </div>
    );
  }

  return (
    <div style={{ padding: '6rem 2rem 2rem 2rem' }}>
      {/* Header */}
      <div style={{ marginBottom: '2rem' }}>
        <h1 style={{
          fontSize: '2rem',
          fontWeight: '600',
          color: '#2d3748',
          marginBottom: '0.5rem'
        }}>
          Users
        </h1>
        <p style={{ color: '#718096' }}>
          Manage system users and their permissions
        </p>
      </div>

      {/* Error Message */}
      {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}

      {/* Main Card */}
      <div style={{
        background: 'white',
        borderRadius: '12px',
        boxShadow: '0 4px 20px rgba(0, 0, 0, 0.05)',
        overflow: 'hidden'
      }}>
        {/* Card Header */}
        <div style={{
          padding: '1.5rem 2rem',
          borderBottom: '1px solid #e2e8f0',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}>
          <h2 style={{
            fontSize: '1.25rem',
            fontWeight: '600',
            color: '#2d3748',
            margin: 0
          }}>
            User List
            {users.length > 0 && (
              <span style={{ 
                marginLeft: '0.5rem', 
                fontSize: '0.875rem', 
                color: '#718096',
                fontWeight: '400'
              }}>
                ({filteredUsers.length} of {users.length})
              </span>
            )}
          </h2>
          <Button variant="primary" onClick={() => setShowCreateModal(true)}>
            + Add User
          </Button>
        </div>

        {/* Filters */}
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0' }}>
          <div style={{
            display: 'flex',
            gap: '1rem',
            flexWrap: 'wrap'
          }}>
            <Input
              placeholder="Search by name, username or email..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              style={{ flex: 1, minWidth: '250px' }}
            />
            <Select
              value={roleFilter}
              onChange={(e) => setRoleFilter(e.target.value)}
              style={{ width: '160px' }}
            >
              <option value="">All Roles</option>
              <option value="Admin">Admin</option>
              <option value="Researcher">Researcher</option>
              <option value="Technician">Technician</option>
            </Select>
          </div>
        </div>

        {/* Table */}
        <div style={{ padding: '1rem 2rem 2rem' }}>
          {loading ? (
            <Loading text="Loading users..." />
          ) : (
            <Table
              data={filteredUsers}
              columns={[
                { 
                  key: 'username', 
                  label: 'Username',
                  render: (item) => (
                    <span style={{ fontWeight: '500', color: '#1a365d' }}>
                      {item.username || '—'}
                    </span>
                  )
                },
                { key: 'name', label: 'Full Name' },
                { key: 'email', label: 'Email' },
                {
                  key: 'role',
                  label: 'Role',
                  render: (item) => {
                    const roleColors = {
                      'Admin': { bg: '#fed7d7', color: '#c53030' },
                      'Researcher': { bg: '#c6f6d5', color: '#2f855a' },
                      'Technician': { bg: '#bee3f8', color: '#2b6cb0' }
                    };
                    const style = roleColors[item.role] || { bg: '#e2e8f0', color: '#4a5568' };
                    return (
                      <Badge style={{ backgroundColor: style.bg, color: style.color }}>
                        {item.role || 'Unknown'}
                      </Badge>
                    );
                  }
                },
                {
                  key: 'is_active',
                  label: 'Status',
                  render: (item) => (
                    <span style={{ 
                      color: item.is_active !== false ? '#38a169' : '#e53e3e',
                      fontWeight: '500'
                    }}>
                      {item.is_active !== false ? '● Active' : '● Inactive'}
                    </span>
                  )
                },
                { 
                  key: 'created_at', 
                  label: 'Created',
                  render: (item) => item.created_at 
                    ? new Date(item.created_at).toLocaleDateString('ru-RU')
                    : '—'
                }
              ]}
              actions={{
                view: true,
                edit: true,
                delete: true,
                reset: true
              }}
              onAction={handleAction}
              emptyMessage={
                users.length === 0 
                  ? "No users found in the system."
                  : "No users match your search criteria."
              }
            />
          )}
        </div>
      </div>

      {/* Modals */}
      {showCreateModal && (
        <CreateUserModal
          isOpen={showCreateModal}
          onClose={() => setShowCreateModal(false)}
          onSave={handleCreateSuccess}
        />
      )}

      {showEditModal && selectedUser && (
        <EditUserModal
          isOpen={showEditModal}
          user={selectedUser}
          onClose={handleCloseEdit}
          onSave={handleEditSuccess}
        />
      )}

      {showViewModal && selectedUser && (
        <ViewUserModal
          isOpen={showViewModal}
          user={selectedUser}
          onClose={handleCloseView}
        />
      )}
    </div>
  );
};

export default Users;
