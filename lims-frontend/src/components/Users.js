// components/Users.js - Full user management with permissions and activity history
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
import { KeyIcon, EditIcon, TrashIcon, EyeIcon } from './Icons';

// ===================== SWITCH COMPONENT =====================
const Switch = ({ checked, onChange, disabled = false, label }) => (
  <label style={{ 
    display: 'flex', 
    alignItems: 'center', 
    gap: '8px',
    cursor: disabled ? 'not-allowed' : 'pointer',
    opacity: disabled ? 0.5 : 1
  }}>
    <div
      onClick={() => !disabled && onChange(!checked)}
      style={{
        width: '44px',
        height: '24px',
        backgroundColor: checked ? '#38a169' : '#cbd5e0',
        borderRadius: '12px',
        position: 'relative',
        transition: 'background-color 0.2s',
        cursor: disabled ? 'not-allowed' : 'pointer'
      }}
    >
      <div style={{
        width: '20px',
        height: '20px',
        backgroundColor: 'white',
        borderRadius: '50%',
        position: 'absolute',
        top: '2px',
        left: checked ? '22px' : '2px',
        transition: 'left 0.2s',
        boxShadow: '0 1px 3px rgba(0,0,0,0.2)'
      }} />
    </div>
    {label && <span style={{ fontSize: '14px', color: '#4a5568' }}>{label}</span>}
  </label>
);

// ===================== ICONS =====================
const ShieldIcon = ({ size = 16 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
  </svg>
);

const HistoryIcon = ({ size = 16 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
  </svg>
);

// ===================== MODAL BASE =====================
const Modal = ({ isOpen, onClose, title, children, width = '500px' }) => {
  if (!isOpen) return null;
  
  return (
    <div style={{
      position: 'fixed',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      backgroundColor: 'rgba(0, 0, 0, 0.5)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 1000
    }} onClick={onClose}>
      <div style={{
        backgroundColor: 'white',
        borderRadius: '12px',
        width,
        maxWidth: '95vw',
        maxHeight: '90vh',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column'
      }} onClick={e => e.stopPropagation()}>
        <div style={{
          padding: '1.5rem',
          borderBottom: '1px solid #e2e8f0',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}>
          <h3 style={{ margin: 0, fontSize: '1.25rem', fontWeight: '600' }}>{title}</h3>
          <button onClick={onClose} style={{ background: 'none', border: 'none', fontSize: '1.5rem', cursor: 'pointer', color: '#718096' }}>×</button>
        </div>
        <div style={{ padding: '1.5rem', overflowY: 'auto', flex: 1 }}>{children}</div>
      </div>
    </div>
  );
};

// ===================== PERMISSIONS MODAL =====================
const PermissionsModal = ({ isOpen, onClose, user, onSave }) => {
  const [permissions, setPermissions] = useState({});
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const permissionGroups = {
    'Reagents': [
      { key: 'create_reagent', label: 'Create Reagents' },
      { key: 'edit_reagent', label: 'Edit Reagents' },
      { key: 'delete_reagent', label: 'Delete Reagents' },
      { key: 'view_reagent', label: 'View Reagents' },
    ],
    'Batches': [
      { key: 'create_batch', label: 'Create Batches' },
      { key: 'edit_batch', label: 'Edit Batches' },
      { key: 'delete_batch', label: 'Delete Batches' },
      { key: 'view_batch', label: 'View Batches' },
      { key: 'use_batch', label: 'Use/Consume Batches' },
    ],
    'Equipment': [
      { key: 'create_equipment', label: 'Create Equipment' },
      { key: 'edit_equipment', label: 'Edit Equipment' },
      { key: 'delete_equipment', label: 'Delete Equipment' },
      { key: 'view_equipment', label: 'View Equipment' },
      { key: 'manage_maintenance', label: 'Manage Maintenance' },
    ],
    'Experiments': [
      { key: 'create_experiment', label: 'Create Experiments' },
      { key: 'edit_experiment', label: 'Edit Experiments' },
      { key: 'delete_experiment', label: 'Delete Experiments' },
      { key: 'view_experiment', label: 'View Experiments' },
    ],
    'Rooms': [
      { key: 'create_room', label: 'Create Rooms' },
      { key: 'edit_room', label: 'Edit Rooms' },
      { key: 'delete_room', label: 'Delete Rooms' },
      { key: 'view_room', label: 'View Rooms' },
    ],
    'Reports & Data': [
      { key: 'view_reports', label: 'View Reports' },
      { key: 'export_reports', label: 'Export Reports' },
      { key: 'import_data', label: 'Import Data' },
      { key: 'export_data', label: 'Export Data' },
    ],
    'System': [
      { key: 'view_audit_log', label: 'View Audit Log' },
      { key: 'manage_users', label: 'Manage Users' },
      { key: 'manage_system', label: 'System Settings' },
    ],
  };

  const rolePresets = {
    admin: Object.values(permissionGroups).flat().reduce((acc, p) => ({ ...acc, [p.key]: true }), {}),
    researcher: {
      create_reagent: true, edit_reagent: true, view_reagent: true,
      create_batch: true, edit_batch: true, view_batch: true, use_batch: true,
      create_equipment: true, edit_equipment: true, view_equipment: true, manage_maintenance: true,
      create_experiment: true, edit_experiment: true, view_experiment: true,
      create_room: true, edit_room: true, view_room: true,
      view_reports: true, export_reports: true, export_data: true,
    },
    viewer: {
      view_reagent: true, view_batch: true, use_batch: true,
      view_equipment: true, view_experiment: true, view_room: true, view_reports: true,
    },
  };

  useEffect(() => {
    if (isOpen && user) {
      setLoading(true);
      api.getUserPermissions(user.id)
        .then(data => setPermissions(data.permissions || rolePresets[user.role?.toLowerCase()] || {}))
        .catch(() => setPermissions(rolePresets[user.role?.toLowerCase()] || {}))
        .finally(() => setLoading(false));
    }
  }, [isOpen, user]);

  const handleToggle = (key) => setPermissions(prev => ({ ...prev, [key]: !prev[key] }));
  const handleApplyPreset = (preset) => setPermissions(rolePresets[preset] || {});
  const handleSelectAll = (group) => {
    const groupKeys = permissionGroups[group].map(p => p.key);
    const allEnabled = groupKeys.every(k => permissions[k]);
    const updates = {};
    groupKeys.forEach(k => updates[k] = !allEnabled);
    setPermissions(prev => ({ ...prev, ...updates }));
  };

  const handleSave = async () => {
    setSaving(true);
    setError('');
    try {
      await api.updateUserPermissions(user.id, permissions);
      onSave && onSave();
      onClose();
    } catch (err) {
      setError(err.message || 'Failed to save permissions');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`Permissions: ${user?.username}`} width="700px">
      {loading ? <Loading text="Loading permissions..." /> : (
        <>
          {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}
          <div style={{ marginBottom: '1.5rem', padding: '1rem', backgroundColor: '#f7fafc', borderRadius: '8px' }}>
            <div style={{ fontSize: '14px', fontWeight: '500', marginBottom: '0.5rem', color: '#4a5568' }}>Quick Presets:</div>
            <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
              <Button variant="secondary" size="small" onClick={() => handleApplyPreset('admin')}>🔑 Full Admin</Button>
              <Button variant="secondary" size="small" onClick={() => handleApplyPreset('researcher')}>🔬 Researcher</Button>
              <Button variant="secondary" size="small" onClick={() => handleApplyPreset('viewer')}>👁️ Viewer Only</Button>
              <Button variant="secondary" size="small" onClick={() => setPermissions({})}>❌ Clear All</Button>
            </div>
          </div>
          <div style={{ display: 'grid', gap: '1.5rem' }}>
            {Object.entries(permissionGroups).map(([group, perms]) => (
              <div key={group} style={{ border: '1px solid #e2e8f0', borderRadius: '8px', overflow: 'hidden' }}>
                <div style={{ padding: '0.75rem 1rem', backgroundColor: '#edf2f7', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontWeight: '600', color: '#2d3748' }}>{group}</span>
                  <Button variant="ghost" size="small" onClick={() => handleSelectAll(group)}>Toggle All</Button>
                </div>
                <div style={{ padding: '1rem', display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '0.75rem' }}>
                  {perms.map(perm => (
                    <Switch key={perm.key} checked={!!permissions[perm.key]} onChange={() => handleToggle(perm.key)} label={perm.label} />
                  ))}
                </div>
              </div>
            ))}
          </div>
          <div style={{ marginTop: '1.5rem', display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
            <Button variant="secondary" onClick={onClose}>Cancel</Button>
            <Button variant="primary" onClick={handleSave} disabled={saving}>{saving ? 'Saving...' : 'Save Permissions'}</Button>
          </div>
        </>
      )}
    </Modal>
  );
};

// ===================== USER ACTIVITY MODAL =====================
const UserActivityModal = ({ isOpen, onClose, user }) => {
  const [activities, setActivities] = useState([]);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState('all');
  const [dateRange, setDateRange] = useState({ from: '', to: '' });

  const loadActivities = useCallback(async () => {
    if (!user) return;
    setLoading(true);
    try {
      const params = { limit: 100, ...(filter !== 'all' && { action_type: filter }), ...(dateRange.from && { date_from: dateRange.from }), ...(dateRange.to && { date_to: dateRange.to }) };
      const data = await api.getUserActivity(user.id, params);
      setActivities(Array.isArray(data) ? data : data.activities || data.data || []);
    } catch (err) {
      console.error('Failed to load activities:', err);
      setActivities([]);
    } finally {
      setLoading(false);
    }
  }, [user, filter, dateRange]);

  // Auto-reload when modal opens OR when filter changes
  useEffect(() => {
    if (isOpen && user) loadActivities();
  }, [isOpen, user, filter, loadActivities]);

  const getActionIcon = (action) => {
    const icons = {
      'create': '➕', 'register': '📝', 'update': '✏️', 'edit': '✏️',
      'delete': '🗑️', 'login': '🔐', 'logout': '🚪', 'view': '👁️',
      'export': '📤', 'import': '📥', 'change_password': '🔑',
      'use_reagent': '🧪', 'jwt_rotation': '🔄'
    };
    const actionLower = action?.toLowerCase() || '';
    for (const [key, icon] of Object.entries(icons)) if (actionLower.includes(key)) return icon;
    return '📋';
  };

  const getActionColor = (action) => {
    const actionLower = action?.toLowerCase() || '';
    if (actionLower.includes('delete')) return '#e53e3e';
    if (actionLower.includes('create') || actionLower.includes('register')) return '#38a169';
    if (actionLower.includes('update') || actionLower.includes('edit') || actionLower.includes('change')) return '#3182ce';
    if (actionLower.includes('login')) return '#805ad5';
    if (actionLower.includes('use_reagent')) return '#d69e2e';
    if (actionLower.includes('jwt') || actionLower.includes('rotation')) return '#ed8936';
    return '#718096';
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return '—';
    return new Date(dateStr).toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' });
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`Activity History: ${user?.username}`} width="800px">
      <div style={{ marginBottom: '1rem', display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'flex-end' }}>
        <div>
          <label style={{ display: 'block', fontSize: '12px', color: '#718096', marginBottom: '4px' }}>Action Type</label>
          <Select value={filter} onChange={(e) => setFilter(e.target.value)} style={{ width: '180px' }}>
            <option value="all">All Actions</option>
            <option value="create">Create</option>
            <option value="update">Update / Edit</option>
            <option value="delete">Delete</option>
            <option value="login">Login</option>
            <option value="register">Register</option>
            <option value="change_password">Change Password</option>
            <option value="use_reagent">Use Reagent</option>
          </Select>
        </div>
        <div>
          <label style={{ display: 'block', fontSize: '12px', color: '#718096', marginBottom: '4px' }}>From Date</label>
          <Input type="date" value={dateRange.from} onChange={(e) => setDateRange(prev => ({ ...prev, from: e.target.value }))} style={{ width: '150px' }} />
        </div>
        <div>
          <label style={{ display: 'block', fontSize: '12px', color: '#718096', marginBottom: '4px' }}>To Date</label>
          <Input type="date" value={dateRange.to} onChange={(e) => setDateRange(prev => ({ ...prev, to: e.target.value }))} style={{ width: '150px' }} />
        </div>
        <Button variant="secondary" onClick={loadActivities}>🔍 Search</Button>
      </div>
      {loading ? <Loading text="Loading activity..." /> : activities.length === 0 ? (
        <div style={{ textAlign: 'center', padding: '3rem', color: '#718096' }}><div style={{ fontSize: '3rem', marginBottom: '1rem' }}>📭</div><p>No activity records found</p></div>
      ) : (
        <div style={{ maxHeight: '500px', overflowY: 'auto' }}>
          {activities.map((activity, index) => (
            <div key={activity.id || index} style={{ padding: '1rem', borderBottom: '1px solid #e2e8f0', display: 'flex', gap: '1rem', alignItems: 'flex-start' }}>
              <div style={{ fontSize: '1.5rem', width: '40px', textAlign: 'center' }}>{getActionIcon(activity.action || activity.action_type)}</div>
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: '500', color: getActionColor(activity.action || activity.action_type), marginBottom: '4px' }}>{activity.action || activity.action_type || 'Unknown Action'}</div>
                <div style={{ fontSize: '14px', color: '#4a5568', marginBottom: '4px' }}>{activity.description || activity.entity_type || '—'}{activity.entity_id && <span style={{ color: '#718096' }}> (ID: {activity.entity_id})</span>}</div>
                {activity.changes && <div style={{ fontSize: '12px', color: '#718096', backgroundColor: '#f7fafc', padding: '0.5rem', borderRadius: '4px', marginTop: '4px' }}><pre style={{ margin: 0, whiteSpace: 'pre-wrap', fontFamily: 'monospace' }}>{typeof activity.changes === 'string' ? activity.changes : JSON.stringify(activity.changes, null, 2)}</pre></div>}
                <div style={{ fontSize: '12px', color: '#a0aec0', marginTop: '4px' }}>{formatDate(activity.created_at || activity.timestamp)}{activity.ip_address && ` • IP: ${activity.ip_address}`}</div>
              </div>
            </div>
          ))}
        </div>
      )}
      {activities.length > 0 && <div style={{ marginTop: '1rem', padding: '1rem', backgroundColor: '#f7fafc', borderRadius: '8px', fontSize: '14px', color: '#4a5568' }}>Showing {activities.length} records</div>}
    </Modal>
  );
};

// ===================== MAIN COMPONENT =====================
const Users = ({ user }) => {
  const [users, setUsers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [roleFilter, setRoleFilter] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showViewModal, setShowViewModal] = useState(false);
  const [showPermissionsModal, setShowPermissionsModal] = useState(false);
  const [showActivityModal, setShowActivityModal] = useState(false);
  const [selectedUser, setSelectedUser] = useState(null);
  const [error, setError] = useState('');

  const isAdmin = user?.role?.toLowerCase() === 'admin';

  const loadUsers = useCallback(async () => {
    if (!isAdmin) return;
    try {
      setError('');
      setLoading(true);
      const data = await api.getUsers();
      let userList = [];
      if (Array.isArray(data)) userList = data;
      else if (data?.users) userList = data.users;
      else if (data?.data) userList = data.data;
      setUsers(userList);
    } catch (err) {
      setError(err.message || 'Failed to load users');
      setUsers([]);
    } finally {
      setLoading(false);
    }
  }, [isAdmin]);

  useEffect(() => { if (isAdmin) loadUsers(); else setLoading(false); }, [isAdmin, loadUsers]);

  const handleAction = useCallback(async (action, item) => {
    try {
      switch (action) {
        case 'view': setSelectedUser(item); setShowViewModal(true); break;
        case 'edit': setSelectedUser(item); setShowEditModal(true); break;
        case 'permissions': setSelectedUser(item); setShowPermissionsModal(true); break;
        case 'activity': setSelectedUser(item); setShowActivityModal(true); break;
        case 'delete':
          if (window.confirm(`Delete user "${item.username}"?\n\nClick OK to DEACTIVATE (recommended)`)) {
            try { await api.deleteUser(item.id, false); loadUsers(); }
            catch (err) {
              if (err.message?.includes('FOREIGN KEY') && window.confirm(`User has related records. Permanently delete?`)) {
                await api.deleteUser(item.id, true); loadUsers();
              } else throw err;
            }
          }
          break;
        case 'reset':
          const newPassword = prompt('Enter new password for user:');
          if (newPassword) {
            if (newPassword.length < 6) { setError('Password must be at least 6 characters'); return; }
            await api.resetUserPassword(item.id, newPassword);
            alert('Password reset successfully');
          }
          break;
        default: console.warn('Unknown action:', action);
      }
    } catch (err) { setError(err.message || `Failed to ${action} user`); }
  }, [loadUsers]);

  const filteredUsers = users.filter(u => {
    if (!u) return false;
    const searchLower = searchTerm.toLowerCase();
    const matchesSearch = !searchTerm || u.email?.toLowerCase().includes(searchLower) || u.name?.toLowerCase().includes(searchLower) || u.username?.toLowerCase().includes(searchLower);
    const matchesRole = !roleFilter || u.role?.toLowerCase() === roleFilter.toLowerCase();
    return matchesSearch && matchesRole;
  });

  if (!isAdmin) return (
    <div style={{ padding: '6rem 2rem 2rem 2rem' }}>
      <div style={{ background: 'white', borderRadius: '12px', boxShadow: '0 4px 20px rgba(0, 0, 0, 0.05)', padding: '2rem', textAlign: 'center' }}>
        <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>🔒</div>
        <h2 style={{ color: '#e53e3e', marginBottom: '0.5rem' }}>Access Denied</h2>
        <p style={{ color: '#718096' }}>Admin privileges required to view this page.</p>
      </div>
    </div>
  );

  return (
    <div style={{ padding: '6rem 2rem 2rem 2rem' }}>
      <div style={{ marginBottom: '2rem' }}>
        <h1 style={{ fontSize: '2rem', fontWeight: '600', color: '#2d3748', marginBottom: '0.5rem' }}>Users</h1>
        <p style={{ color: '#718096' }}>Manage system users, permissions, and view activity history</p>
      </div>
      {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}
      <div style={{ background: 'white', borderRadius: '12px', boxShadow: '0 4px 20px rgba(0, 0, 0, 0.05)', overflow: 'hidden' }}>
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h2 style={{ fontSize: '1.25rem', fontWeight: '600', color: '#2d3748', margin: 0 }}>User List {users.length > 0 && <span style={{ marginLeft: '0.5rem', fontSize: '0.875rem', color: '#718096', fontWeight: '400' }}>({filteredUsers.length} of {users.length})</span>}</h2>
          <Button variant="primary" onClick={() => setShowCreateModal(true)}>+ Add User</Button>
        </div>
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0' }}>
          <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap' }}>
            <Input placeholder="Search by name, username or email..." value={searchTerm} onChange={(e) => setSearchTerm(e.target.value)} style={{ flex: 1, minWidth: '250px' }} />
            <Select value={roleFilter} onChange={(e) => setRoleFilter(e.target.value)} style={{ width: '160px' }}>
              <option value="">All Roles</option>
              <option value="admin">Admin</option>
              <option value="researcher">Researcher</option>
              <option value="viewer">Viewer</option>
            </Select>
          </div>
        </div>
        <div style={{ padding: '1rem 2rem 2rem' }}>
          {loading ? <Loading text="Loading users..." /> : (
            <Table data={filteredUsers} columns={[
              { key: 'username', label: 'Username', render: (item) => <span style={{ fontWeight: '500', color: '#1a365d' }}>{item.username || '—'}</span> },
              { key: 'name', label: 'Full Name' },
              { key: 'email', label: 'Email' },
              { key: 'role', label: 'Role', render: (item) => {
                const role = item.role?.toLowerCase();
                const roleColors = { 'admin': { bg: '#fed7d7', color: '#c53030' }, 'researcher': { bg: '#c6f6d5', color: '#2f855a' }, 'viewer': { bg: '#bee3f8', color: '#2b6cb0' } };
                const style = roleColors[role] || { bg: '#e2e8f0', color: '#4a5568' };
                return <Badge style={{ backgroundColor: style.bg, color: style.color }}>{role ? role.charAt(0).toUpperCase() + role.slice(1) : 'Unknown'}</Badge>;
              }},
              { key: 'is_active', label: 'Status', render: (item) => <span style={{ color: item.is_active !== false ? '#38a169' : '#e53e3e', fontWeight: '500' }}>{item.is_active !== false ? '● Active' : '● Inactive'}</span> },
              { key: 'created_at', label: 'Created', render: (item) => item.created_at ? new Date(item.created_at).toLocaleDateString('ru-RU') : '—' },
              { key: 'actions', label: 'Actions', render: (item) => (
                <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                  <Button variant="secondary" onClick={() => handleAction('view', item)} style={{ padding: '6px', display: 'flex', alignItems: 'center' }} title="View Details"><EyeIcon size={16} /></Button>
                  <Button variant="secondary" onClick={() => handleAction('edit', item)} style={{ padding: '6px', display: 'flex', alignItems: 'center' }} title="Edit User"><EditIcon size={16} /></Button>
                  <Button onClick={() => handleAction('permissions', item)} style={{ padding: '6px', display: 'flex', alignItems: 'center', backgroundColor: '#805ad5', color: 'white', border: 'none', borderRadius: '6px', cursor: 'pointer' }} title="Manage Permissions"><ShieldIcon size={16} /></Button>
                  <Button onClick={() => handleAction('activity', item)} style={{ padding: '6px', display: 'flex', alignItems: 'center', backgroundColor: '#3182ce', color: 'white', border: 'none', borderRadius: '6px', cursor: 'pointer' }} title="View Activity History"><HistoryIcon size={16} /></Button>
                  <Button onClick={() => handleAction('reset', item)} style={{ padding: '6px', display: 'flex', alignItems: 'center', backgroundColor: '#ecc94b', color: 'white', border: 'none', borderRadius: '6px', cursor: 'pointer' }} title="Reset Password"><KeyIcon size={16} /></Button>
                  <Button variant="danger" onClick={() => handleAction('delete', item)} style={{ padding: '6px', display: 'flex', alignItems: 'center' }} title="Delete User"><TrashIcon size={16} /></Button>
                </div>
              )}
            ]} emptyMessage={users.length === 0 ? "No users found in the system." : "No users match your search criteria."} />
          )}
        </div>
      </div>
      {showCreateModal && <CreateUserModal isOpen={showCreateModal} onClose={() => setShowCreateModal(false)} onSave={() => { setShowCreateModal(false); loadUsers(); }} />}
      {showEditModal && selectedUser && <EditUserModal isOpen={showEditModal} user={selectedUser} onClose={() => { setShowEditModal(false); setSelectedUser(null); }} onSave={() => { setShowEditModal(false); setSelectedUser(null); loadUsers(); }} />}
      {showViewModal && selectedUser && <ViewUserModal isOpen={showViewModal} user={selectedUser} onClose={() => { setShowViewModal(false); setSelectedUser(null); }} />}
      {showPermissionsModal && selectedUser && <PermissionsModal isOpen={showPermissionsModal} user={selectedUser} onClose={() => { setShowPermissionsModal(false); setSelectedUser(null); }} onSave={loadUsers} />}
      {showActivityModal && selectedUser && <UserActivityModal isOpen={showActivityModal} user={selectedUser} onClose={() => { setShowActivityModal(false); setSelectedUser(null); }} />}
    </div>
  );
};

export default Users;
