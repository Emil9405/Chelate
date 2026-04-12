// src/components/ArchivePage.js
// Archive (Trash) page — view, restore, and permanently delete items
// Admin-only access

import React, { useState, useEffect, useCallback } from 'react';
import { api } from '../services/api';
import { ChevronLeftIcon } from './Icons';

// ============================================================
//                       STYLES
// ============================================================

const styles = {
  container: {
    padding: '6rem 2rem 2rem 2rem',
    maxWidth: '1200px',
    margin: '0 auto',
  },
  backButton: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '6px',
    padding: '8px 16px',
    borderRadius: '8px',
    border: '1px solid #e2e8f0',
    backgroundColor: '#fff',
    color: '#4a5568',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 500,
    marginBottom: '20px',
    transition: 'all 0.2s',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '24px',
  },
  title: {
    fontSize: '2rem',
    fontWeight: 600,
    color: '#2d3748',
    margin: 0,
  },
  subtitle: {
    color: '#718096',
    marginTop: '4px',
    fontSize: '14px',
  },
  statsRow: {
    display: 'flex',
    gap: '12px',
  },
  statBadge: {
    padding: '6px 14px',
    borderRadius: '20px',
    fontSize: '13px',
    fontWeight: 500,
    backgroundColor: '#edf2f7',
    color: '#4a5568',
  },
  tabs: {
    display: 'flex',
    gap: '0',
    marginBottom: '20px',
    borderBottomWidth: '2px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#e2e8f0',
  },
  tab: {
    padding: '10px 24px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 500,
    color: '#718096',
    backgroundColor: 'transparent',
    border: 'none',
    borderBottomWidth: '2px',
    borderBottomStyle: 'solid',
    borderBottomColor: 'transparent',
    marginBottom: '-2px',
    transition: 'all 0.2s',
  },
  tabActive: {
    color: '#3182ce',
    borderBottomColor: '#3182ce',
  },
  tableWrap: {
    background: 'white',
    borderRadius: '12px',
    boxShadow: '0 4px 20px rgba(0, 0, 0, 0.05)',
    overflow: 'hidden',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '14px',
  },
  th: {
    textAlign: 'left',
    padding: '12px 16px',
    backgroundColor: '#f7fafc',
    color: '#4a5568',
    fontWeight: 600,
    fontSize: '12px',
    textTransform: 'uppercase',
    letterSpacing: '0.5px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#e2e8f0',
  },
  td: {
    padding: '12px 16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#edf2f7',
    color: '#2d3748',
  },
  row: {
    transition: 'background-color 0.15s',
  },
  emptyState: {
    textAlign: 'center',
    padding: '60px 20px',
    color: '#a0aec0',
  },
  emptyIcon: {
    fontSize: '48px',
    marginBottom: '12px',
  },
  actions: {
    display: 'flex',
    gap: '8px',
  },
  btnRestore: {
    padding: '6px 14px',
    borderRadius: '6px',
    border: 'none',
    cursor: 'pointer',
    fontSize: '13px',
    fontWeight: 500,
    backgroundColor: '#c6f6d5',
    color: '#276749',
    transition: 'all 0.2s',
  },
  btnDelete: {
    padding: '6px 14px',
    borderRadius: '6px',
    border: 'none',
    cursor: 'pointer',
    fontSize: '13px',
    fontWeight: 500,
    backgroundColor: '#fed7d7',
    color: '#9b2c2c',
    transition: 'all 0.2s',
  },
  btnDisabled: {
    opacity: 0.5,
    cursor: 'not-allowed',
  },
  statusBadge: {
    display: 'inline-block',
    padding: '2px 10px',
    borderRadius: '10px',
    fontSize: '12px',
    fontWeight: 500,
  },
  loading: {
    textAlign: 'center',
    padding: '40px',
    color: '#a0aec0',
  },
  error: {
    padding: '16px',
    backgroundColor: '#fed7d7',
    color: '#9b2c2c',
    borderRadius: '8px',
    marginBottom: '16px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  // Modal
  modalOverlay: {
    position: 'fixed',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 1000,
  },
  modal: {
    backgroundColor: '#fff',
    borderRadius: '12px',
    padding: '28px',
    maxWidth: '440px',
    width: '90%',
    boxShadow: '0 20px 60px rgba(0, 0, 0, 0.3)',
  },
  modalTitle: {
    fontSize: '18px',
    fontWeight: 600,
    marginBottom: '12px',
    color: '#2d3748',
  },
  modalText: {
    fontSize: '14px',
    color: '#4a5568',
    lineHeight: 1.6,
    marginBottom: '24px',
  },
  modalActions: {
    display: 'flex',
    justifyContent: 'flex-end',
    gap: '10px',
  },
  btnCancel: {
    padding: '8px 20px',
    borderRadius: '6px',
    border: '1px solid #e2e8f0',
    backgroundColor: '#fff',
    color: '#4a5568',
    cursor: 'pointer',
    fontSize: '14px',
  },
  btnConfirmDelete: {
    padding: '8px 20px',
    borderRadius: '6px',
    border: 'none',
    backgroundColor: '#e53e3e',
    color: '#fff',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 500,
  },
  // Toast
  toast: {
    position: 'fixed',
    bottom: '24px',
    right: '24px',
    padding: '14px 24px',
    borderRadius: '8px',
    color: '#fff',
    fontSize: '14px',
    fontWeight: 500,
    zIndex: 1001,
    boxShadow: '0 4px 20px rgba(0, 0, 0, 0.15)',
  },
  toastSuccess: {
    backgroundColor: '#38a169',
  },
  toastError: {
    backgroundColor: '#e53e3e',
  },
};

// ============================================================
//                    CONFIRM MODAL
// ============================================================

function ConfirmModal({ isOpen, title, message, onConfirm, onCancel, isLoading }) {
  if (!isOpen) return null;

  return (
    <div style={styles.modalOverlay} onClick={onCancel}>
      <div style={styles.modal} onClick={e => e.stopPropagation()}>
        <div style={styles.modalTitle}>{title}</div>
        <div style={styles.modalText}>{message}</div>
        <div style={styles.modalActions}>
          <button
            style={styles.btnCancel}
            onClick={onCancel}
            disabled={isLoading}
          >
            Cancel
          </button>
          <button
            style={{
              ...styles.btnConfirmDelete,
              ...(isLoading ? styles.btnDisabled : {}),
            }}
            onClick={onConfirm}
            disabled={isLoading}
          >
            {isLoading ? 'Deleting...' : 'Delete Permanently'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================================
//                      TOAST
// ============================================================

function Toast({ message, type, onClose }) {
  useEffect(() => {
    const timer = setTimeout(onClose, 3500);
    return () => clearTimeout(timer);
  }, [onClose]);

  if (!message) return null;

  return (
    <div style={{
      ...styles.toast,
      ...(type === 'success' ? styles.toastSuccess : styles.toastError),
    }}>
      {type === 'success' ? '✓ ' : '✕ '}{message}
    </div>
  );
}

// ============================================================
//                     HELPERS
// ============================================================

function formatDate(dateStr) {
  if (!dateStr) return '—';
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString('en-GB', {
      day: '2-digit', month: '2-digit', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  } catch {
    return dateStr;
  }
}

function getStatusStyle(status) {
  const map = {
    inactive:  { backgroundColor: '#fefcbf', color: '#975a16' },
    retired:   { backgroundColor: '#bee3f8', color: '#2b6cb0' },
    expired:   { backgroundColor: '#fed7d7', color: '#9b2c2c' },
    disposed:  { backgroundColor: '#e9d8fd', color: '#6b46c1' },
  };
  return map[status] || { backgroundColor: '#edf2f7', color: '#4a5568' };
}

// ============================================================
//                   MAIN COMPONENT
// ============================================================

export default function ArchivePage({ user, onNavigateBack }) {
  const [activeTab, setActiveTab] = useState('reagents');
  const [reagents, setReagents] = useState([]);
  const [equipment, setEquipment] = useState([]);
  const [stats, setStats] = useState({ reagents_count: 0, equipment_count: 0, total: 0 });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [actionLoading, setActionLoading] = useState(null);
  const [toast, setToast] = useState({ message: '', type: '' });
  const [confirmModal, setConfirmModal] = useState({
    isOpen: false, item: null, entityType: '',
  });

  // ---- Fetch data ----

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [statsData, reagentsData, equipmentData] = await Promise.all([
        api.getArchiveStats(),
        api.getArchivedReagents(),
        api.getArchivedEquipment(),
      ]);
      setStats(statsData);
      setReagents(reagentsData);
      setEquipment(equipmentData);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // ---- Actions ----

  const handleRestore = async (id, entityType) => {
    setActionLoading(id);
    try {
      await api.archiveRestore(id, entityType);
      setToast({ message: 'Restored successfully', type: 'success' });
      if (entityType === 'reagent') {
        setReagents(prev => prev.filter(r => r.id !== id));
        setStats(prev => ({ ...prev, reagents_count: prev.reagents_count - 1, total: prev.total - 1 }));
      } else {
        setEquipment(prev => prev.filter(e => e.id !== id));
        setStats(prev => ({ ...prev, equipment_count: prev.equipment_count - 1, total: prev.total - 1 }));
      }
    } catch (err) {
      setToast({ message: `Error: ${err.message}`, type: 'error' });
    } finally {
      setActionLoading(null);
    }
  };

  const openDeleteConfirm = (item, entityType) => {
    setConfirmModal({ isOpen: true, item, entityType });
  };

  const handleHardDelete = async () => {
    const { item, entityType } = confirmModal;
    setActionLoading(item.id);
    setConfirmModal({ isOpen: false, item: null, entityType: '' });
    try {
      await api.archiveHardDelete(item.id, entityType);
      setToast({ message: 'Permanently deleted', type: 'success' });
      if (entityType === 'reagent') {
        setReagents(prev => prev.filter(r => r.id !== item.id));
        setStats(prev => ({ ...prev, reagents_count: prev.reagents_count - 1, total: prev.total - 1 }));
      } else {
        setEquipment(prev => prev.filter(e => e.id !== item.id));
        setStats(prev => ({ ...prev, equipment_count: prev.equipment_count - 1, total: prev.total - 1 }));
      }
    } catch (err) {
      setToast({ message: `Error: ${err.message}`, type: 'error' });
    } finally {
      setActionLoading(null);
    }
  };

  // ---- Render ----

  const currentItems = activeTab === 'reagents' ? reagents : equipment;

  return (
    <div style={styles.container}>
      {/* Back button */}
      {onNavigateBack && (
        <button
          style={styles.backButton}
          onClick={onNavigateBack}
          onMouseEnter={e => { e.currentTarget.style.backgroundColor = '#edf2f7'; }}
          onMouseLeave={e => { e.currentTarget.style.backgroundColor = '#fff'; }}
        >
          <ChevronLeftIcon size={16} /> Back
        </button>
      )}

      {/* Header */}
      <div style={styles.header}>
        <div>
          <h1 style={styles.title}>Archive</h1>
          <p style={styles.subtitle}>Deleted reagents and equipment — restore or permanently remove</p>
        </div>
        <div style={styles.statsRow}>
          <span style={styles.statBadge}>
            Reagents: {stats.reagents_count}
          </span>
          <span style={styles.statBadge}>
            Equipment: {stats.equipment_count}
          </span>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div style={styles.error}>
          <span>{error}</span>
          <button
            onClick={loadData}
            style={{ cursor: 'pointer', textDecoration: 'underline', background: 'none', border: 'none', color: 'inherit', fontWeight: 500 }}
          >
            Retry
          </button>
        </div>
      )}

      {/* Tabs */}
      <div style={styles.tabs}>
        <button
          style={{ ...styles.tab, ...(activeTab === 'reagents' ? styles.tabActive : {}) }}
          onClick={() => setActiveTab('reagents')}
        >
          Reagents ({stats.reagents_count})
        </button>
        <button
          style={{ ...styles.tab, ...(activeTab === 'equipment' ? styles.tabActive : {}) }}
          onClick={() => setActiveTab('equipment')}
        >
          Equipment ({stats.equipment_count})
        </button>
      </div>

      {/* Content */}
      <div style={styles.tableWrap}>
        {loading ? (
          <div style={styles.loading}>Loading archive...</div>
        ) : currentItems.length === 0 ? (
          <div style={styles.emptyState}>
            <div style={styles.emptyIcon}>📦</div>
            <div style={{ fontSize: '16px', fontWeight: 500, marginBottom: '4px' }}>Archive is empty</div>
            <div style={{ fontSize: '13px' }}>
              Deleted {activeTab === 'reagents' ? 'reagents' : 'equipment'} will appear here
            </div>
          </div>
        ) : (
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>Name</th>
                {activeTab === 'reagents' && <th style={styles.th}>Formula</th>}
                {activeTab === 'reagents' && <th style={styles.th}>CAS</th>}
                {activeTab === 'equipment' && <th style={styles.th}>Type</th>}
                {activeTab === 'equipment' && <th style={styles.th}>S/N</th>}
                <th style={styles.th}>Status</th>
                <th style={styles.th}>Deleted</th>
                <th style={styles.th}>Deleted By</th>
                <th style={styles.th}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {currentItems.map(item => (
                <tr
                  key={item.id}
                  style={styles.row}
                  onMouseEnter={e => e.currentTarget.style.backgroundColor = '#f7fafc'}
                  onMouseLeave={e => e.currentTarget.style.backgroundColor = ''}
                >
                  <td style={{ ...styles.td, fontWeight: 500 }}>
                    {item.name}
                    {activeTab === 'reagents' && item.deleted_batches_count > 0 && (
                      <span style={{ fontSize: '11px', color: '#a0aec0', marginLeft: '6px' }}>
                        ({item.deleted_batches_count} batch{item.deleted_batches_count !== 1 ? 'es' : ''})
                      </span>
                    )}
                  </td>
                  {activeTab === 'reagents' && (
                    <td style={{ ...styles.td, fontStyle: 'italic', color: '#718096' }}>
                      {item.formula || '—'}
                    </td>
                  )}
                  {activeTab === 'reagents' && (
                    <td style={{ ...styles.td, fontFamily: 'monospace', fontSize: '13px' }}>
                      {item.cas_number || '—'}
                    </td>
                  )}
                  {activeTab === 'equipment' && (
                    <td style={styles.td}>{item.type_ || '—'}</td>
                  )}
                  {activeTab === 'equipment' && (
                    <td style={{ ...styles.td, fontFamily: 'monospace', fontSize: '13px' }}>
                      {item.serial_number || '—'}
                    </td>
                  )}
                  <td style={styles.td}>
                    <span style={{ ...styles.statusBadge, ...getStatusStyle(item.status) }}>
                      {item.status}
                    </span>
                  </td>
                  <td style={{ ...styles.td, fontSize: '13px', color: '#718096' }}>
                    {formatDate(item.deleted_at)}
                  </td>
                  <td style={{ ...styles.td, fontSize: '13px' }}>
                    {item.deleted_by_name || item.updated_by || '—'}
                  </td>
                  <td style={styles.td}>
                    <div style={styles.actions}>
                      <button
                        style={{
                          ...styles.btnRestore,
                          ...(actionLoading === item.id ? styles.btnDisabled : {}),
                        }}
                        onClick={() => handleRestore(item.id, activeTab === 'reagents' ? 'reagent' : 'equipment')}
                        disabled={actionLoading === item.id}
                        title="Restore"
                      >
                        {actionLoading === item.id ? '...' : '♻ Restore'}
                      </button>
                      <button
                        style={{
                          ...styles.btnDelete,
                          ...(actionLoading === item.id ? styles.btnDisabled : {}),
                        }}
                        onClick={() => openDeleteConfirm(item, activeTab === 'reagents' ? 'reagent' : 'equipment')}
                        disabled={actionLoading === item.id}
                        title="Delete permanently"
                      >
                        ✕
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Confirm Modal */}
      <ConfirmModal
        isOpen={confirmModal.isOpen}
        title="Delete Permanently?"
        message={
          confirmModal.item
            ? `Are you sure you want to permanently delete "${confirmModal.item.name}"? This action cannot be undone.${
                confirmModal.entityType === 'reagent'
                  ? ' All associated batches, containers, and usage records will be removed.'
                  : ' All associated details, maintenance records, and files will be removed.'
              }`
            : ''
        }
        onConfirm={handleHardDelete}
        onCancel={() => setConfirmModal({ isOpen: false, item: null, entityType: '' })}
        isLoading={actionLoading !== null}
      />

      {/* Toast */}
      {toast.message && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={() => setToast({ message: '', type: '' })}
        />
      )}
    </div>
  );
}
