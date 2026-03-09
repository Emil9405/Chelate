// src/components/Reagents/index.js
import React, { useState, useEffect, useMemo } from 'react';
import useReagents from '../hooks/useReagents';
import { api } from '../../services/api';
import ErrorMessage from '../ErrorMessage';
import Loading from '../Loading';
import Button from '../Button';
import Input from '../Input';
import Select from '../Select';
import ReagentAccordionItem from './ReagentAccordionItem';
import { COLUMN_WIDTHS } from './styles';
import LocationPicker from '../storage/LocationPicker'; // <-- ИМПОРТ LOCATION PICKER

import {
  CreateReagentModal,
  EditReagentModal,
  ViewReagentModal
} from '../Modals';
import BatchImportModal from '../BatchImportModal';

import {
  SearchIcon,
  FilterIcon,
  PlusIcon,
  UploadIcon,
  ChevronRightIcon,
  ChevronLeftIcon,
  CloseIcon,
  FlaskIcon
} from '../Icons';

const Reagents = ({ user }) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [manufacturerFilter, setManufacturerFilter] = useState('');
  const [stockFilter, setStockFilter] = useState('');
  const [casNumberFilter, setCasNumberFilter] = useState('');
  const [positionFilter, setPositionFilter] = useState(null); // <-- СОСТОЯНИЕ ЛОКАЦИИ
  const [sortBy, setSortBy] = useState('created_at');
  const [sortOrder, setSortOrder] = useState('desc');
  const [showFilters, setShowFilters] = useState(false);

  const activeFilters = useMemo(() => {
    const filters = {};
    if (searchTerm) filters.search = searchTerm;
    if (statusFilter) filters.status = statusFilter;
    if (manufacturerFilter) filters.manufacturer = manufacturerFilter;
    if (stockFilter) filters.stock_status = stockFilter;
    if (casNumberFilter) filters.cas_number = casNumberFilter;
    if (positionFilter) filters.position_id = positionFilter; // <-- ФИЛЬТР ПО ЛОКАЦИИ
    return filters;
  }, [searchTerm, statusFilter, manufacturerFilter, stockFilter, casNumberFilter, positionFilter]);

  const {
    data: reagents,
    loading,
    error,
    pagination,
    sorting,
    refresh,
    actions
  } = useReagents(activeFilters, {
    initialPerPage: 20,
    useCursor: false
  });

  useEffect(() => {
    if (sorting.setSortFull) {
      sorting.setSortFull(sortBy, sortOrder);
    } else {
      sorting.setSort(sortBy);
    }
  }, [sortBy, sortOrder]);

  const [expandedReagents, setExpandedReagents] = useState(new Set());
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showViewModal, setShowViewModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [selectedReagent, setSelectedReagent] = useState(null);

  const [allBatches, setAllBatches] = useState([]);
  const [manufacturers, setManufacturers] = useState(['Sigma-Aldrich', 'Merck', 'Thermo Fisher', 'VWR', 'Alfa Aesar']);

  useEffect(() => {
    if (reagents.length > 0) {
      const uniqueManufacturers = [...new Set(reagents.map(r => r.manufacturer).filter(Boolean))].sort();
      setManufacturers(prev => [...new Set([...prev, ...uniqueManufacturers])].sort());
    }
  }, [reagents]);

  useEffect(() => {
    const loadReferenceData = async () => {
      try {
        const batchesData = await api.getAllBatches();
        setAllBatches(Array.isArray(batchesData) ? batchesData : (batchesData.data || []));
      } catch (e) {
        console.error("Failed to load reference data:", e);
      }
    };
    loadReferenceData();
  }, []);

  const userRole = (user?.role || '').toString().toLowerCase();
  const isAdmin = userRole === 'admin';
  const isResearcher = userRole === 'researcher';
  
  const hasPermission = (permissionKey) => {
    if (isAdmin) return true;
    const perms = user?.permissions;
    if (!perms) return false;
    if (Array.isArray(perms)) return perms.includes(permissionKey);
    if (typeof perms === 'object') return perms[permissionKey] === true;
    return false;
  };
  
  const canEditReagents = () => isAdmin || isResearcher || hasPermission('edit_reagent');
  const canDeleteReagents = () => hasPermission('delete_reagent');
  const canDeleteBatches = () => hasPermission('delete_batch');

  const handleAction = async (action, reagent) => {
    switch (action) {
      case 'view':
        setSelectedReagent(reagent);
        setShowViewModal(true);
        break;
      case 'edit':
        setSelectedReagent(reagent);
        setShowEditModal(true);
        break;
      case 'delete':
        if (window.confirm(`Delete reagent "${reagent.name}"? This will also delete all associated batches.`)) {
          try {
            await api.deleteReagent(reagent.id);
            actions.removeItem(reagent.id);
          } catch (err) {
            alert(err.message || 'Failed to delete reagent');
          }
        }
        break;
      default: break;
    }
  };

  const handleCreateSuccess = () => {
    setShowCreateModal(false);
    refresh();
  };

  const handleEditSuccess = () => {
    setShowEditModal(false);
    setSelectedReagent(null);
    refresh();
  };

  const handleImport = async (importData) => {
    try {
      if (importData instanceof File) {
        await api.importReagents(importData);
      } else {
        const blob = new Blob([JSON.stringify(importData)], { type: 'application/json' });
        const file = new File([blob], 'import.json', { type: 'application/json' });
        await api.importReagents(file);
      }
      setShowImportModal(false);
      refresh();
    } catch (err) {
      console.error('Failed to import:', err);
      throw err;
    }
  };

  const toggleAccordion = (reagentId) => {
    setExpandedReagents(prev => {
      const newSet = new Set(prev);
      if (newSet.has(reagentId)) newSet.delete(reagentId);
      else newSet.add(reagentId);
      return newSet;
    });
  };

  const clearFilters = () => {
    setSearchTerm('');
    setStatusFilter('');
    setManufacturerFilter('');
    setStockFilter('');
    setCasNumberFilter('');
    setPositionFilter(null); // <-- Очистка локации
    setSortBy('created_at');
    setSortOrder('desc');
  };

  const activeFiltersCount = [searchTerm, statusFilter, manufacturerFilter, stockFilter, casNumberFilter, positionFilter].filter(Boolean).length;

  if (error) return <ErrorMessage message={error} onRetry={refresh} />;

  return (
    <div style={{ padding: '20px', paddingTop: '90px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' }}>
        <div>
          <h1 style={{ margin: 0, fontSize: '1.75rem', fontWeight: '800', color: '#1a365d' }}>Reagents</h1>
          <p style={{ margin: '4px 0 0 0', color: '#718096', fontSize: '0.875rem' }}>Manage your laboratory reagents and batches</p>
        </div>
        <div style={{ display: 'flex', gap: '10px' }}>
          <Button variant="secondary" onClick={() => setShowImportModal(true)} icon={<UploadIcon size={16} />}>Import</Button>
          {canEditReagents() && (
              <Button variant="primary" onClick={() => setShowCreateModal(true)} icon={<PlusIcon size={16} />}>Add Reagent</Button>
          )}
        </div>
      </div>

      <div style={{ marginBottom: '20px' }}>
        {/* Панель поиска и фильтров */}
        <div style={{ display: 'flex', gap: '10px', marginBottom: '10px' }}>
          <div style={{ flex: 1, position: 'relative' }}>
            <div style={{ position: 'absolute', left: '14px', top: '50%', transform: 'translateY(-50%)', pointerEvents: 'none' }}>
              <SearchIcon size={18} color="#a0aec0" />
            </div>
            <Input
              type="text"
              placeholder="Search reagents by name, CAS, formula..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              style={{ paddingLeft: '44px' }}
            />
          </div>
          <Button variant="secondary" onClick={() => setShowFilters(!showFilters)} icon={<FilterIcon size={16} />}>
            {showFilters ? 'Hide' : 'Filters'}
            {activeFiltersCount > 0 && (
              <span style={{ marginLeft: '6px', background: 'linear-gradient(135deg, #3182ce, #38a169)', color: 'white', borderRadius: '10px', padding: '2px 8px', fontSize: '0.7rem', fontWeight: '700' }}>
                {activeFiltersCount}
              </span>
            )}
          </Button>
          {activeFiltersCount > 0 && <Button variant="ghost" onClick={clearFilters} icon={<CloseIcon size={16} />}>Clear</Button>}
        </div>

        {showFilters && (
          <div style={{ backgroundColor: '#fff', padding: '20px', borderRadius: '12px', marginBottom: '16px', border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(26, 54, 93, 0.06)' }}>
            {/* ИЗМЕНЕНИЕ: Увеличил minmax до 200px чтобы LocationPicker влезал красиво */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Status</label>
                <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
                  <option value="">All Statuses</option>
                  <option value="active">Active</option>
                  <option value="inactive">Inactive</option>
                </Select>
              </div>
              
              {/* ВНЕДРЕНИЕ LOCATION PICKER */}
              <div>
                <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Location</label>
                <LocationPicker
                  value={positionFilter}
                  onChange={setPositionFilter}
                  showLabel={false}
                  placeholder="All locations"
                />
              </div>

              <div>
                <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Manufacturer</label>
                <Select value={manufacturerFilter} onChange={(e) => setManufacturerFilter(e.target.value)}>
                  <option value="">All Manufacturers</option>
                  {manufacturers.map(m => <option key={m} value={m}>{m}</option>)}
                </Select>
              </div>
              <div>
                <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Sort By</label>
                <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
                  <option value="created_at">Date Added</option>
                  <option value="name">Name</option>
                  <option value="total_quantity">Total Quantity</option>
                </Select>
              </div>
              <div>
                <label style={{ display: 'block', marginBottom: '6px', fontWeight: '600', fontSize: '0.8rem', color: '#1a365d' }}>Sort Order</label>
                <Select value={sortOrder} onChange={(e) => setSortOrder(e.target.value)}>
                  <option value="desc">Descending</option>
                  <option value="asc">Ascending</option>
                </Select>
              </div>
            </div>
          </div>
        )}
      </div>

      <div style={{ marginBottom: '15px', color: '#718096', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>Showing <strong style={{ color: '#1a365d' }}>{reagents.length}</strong> of <strong style={{ color: '#1a365d' }}>{pagination.total}</strong> reagents</span>
      </div>

      <div className="reagents-content">
        <div style={{ display: 'flex', padding: '14px 18px', background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.08) 0%, rgba(56, 161, 105, 0.08) 100%)', borderRadius: '12px 12px 0 0', fontWeight: '700', fontSize: '0.7rem', color: '#1a365d', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '2px', gap: '14px' }}>
          <div style={{ width: COLUMN_WIDTHS.expandIcon, flexShrink: 0 }}></div>
          <div style={{ flex: 1, display: 'grid', gridTemplateColumns: COLUMN_WIDTHS.gridColumns, gap: '12px' }}>
            <div>Name</div><div>Formula</div><div>MW</div><div>CAS</div><div>Status</div><div>Stock</div>
          </div>
          <div style={{ width: COLUMN_WIDTHS.actions, flexShrink: 0 }}>Actions</div>
        </div>

        {reagents.length === 0 && !loading ? (
          <div style={{ textAlign: 'center', padding: '60px 40px', color: '#718096', backgroundColor: '#fff', borderRadius: '0 0 12px 12px', border: '1px solid #e2e8f0', borderTop: 'none' }}>
            <div style={{ width: '80px', height: '80px', margin: '0 auto 20px', background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.1) 0%, rgba(56, 161, 105, 0.1) 100%)', borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <FlaskIcon size={32} color="#3182ce" />
            </div>
            <h3 style={{ margin: '0 0 8px 0', color: '#1a365d', fontSize: '1.1rem' }}>
              {activeFiltersCount > 0 ? 'No matches found' : 'No reagents found'}
            </h3>
            <p style={{ margin: '0 0 20px 0', fontSize: '0.9rem' }}>
              {activeFiltersCount > 0 ? "Try adjusting your filters." : "Get started by adding your first reagent."}
            </p>
          </div>
        ) : (
          reagents.map(reagent => (
            <ReagentAccordionItem
              key={reagent.id}
              reagent={reagent}
              isExpanded={expandedReagents.has(reagent.id)}
              onToggle={() => toggleAccordion(reagent.id)}
              onAction={handleAction}
              onReagentsRefresh={refresh}
              canEdit={canEditReagents()}
              canDelete={canDeleteReagents()}
              canDeleteBatch={canDeleteBatches()}
            />
          ))
        )}

        {loading && <div style={{ padding: '20px', textAlign: 'center' }}><Loading /></div>}

        {!loading && reagents.length > 0 && (
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '16px', marginTop: '20px', background: 'white', borderRadius: '8px', border: '1px solid #e2e8f0' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
              <div style={{ fontSize: '0.9rem', color: '#4a5568' }}>
                Page <b>{pagination.page}</b> of <b>{pagination.totalPages}</b>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', borderLeft: '1px solid #e2e8f0', paddingLeft: '16px' }}>
                <span style={{ fontSize: '0.8rem', color: '#718096' }}>Rows:</span>
                <div style={{ width: '80px' }}>
                  <Select
                    value={pagination.perPage}
                    onChange={(e) => pagination.setPerPage(e.target.value)}
                    options={[ { value: 10, label: '10' }, { value: 20, label: '20' }, { value: 50, label: '50' }, { value: 100, label: '100' } ]}
                  />
                </div>
              </div>
            </div>

            <div style={{ display: 'flex', gap: '8px' }}>
              <Button variant="outline" onClick={pagination.goPrev} disabled={pagination.page <= 1} icon={<ChevronLeftIcon />}>
                Previous
              </Button>
              <Button variant="outline" onClick={pagination.goNext} disabled={!pagination.hasNext && pagination.page >= pagination.totalPages}>
                Next <ChevronRightIcon />
              </Button>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && <CreateReagentModal isOpen={showCreateModal} onClose={() => setShowCreateModal(false)} onSave={handleCreateSuccess} />}
      {showEditModal && selectedReagent && <EditReagentModal isOpen={showEditModal} reagent={selectedReagent} onClose={() => { setShowEditModal(false); setSelectedReagent(null); }} onSave={handleEditSuccess} />}
      {showViewModal && selectedReagent && <ViewReagentModal isOpen={showViewModal} onClose={() => { setShowViewModal(false); setSelectedReagent(null); }} reagent={selectedReagent} />}
      {showImportModal && <BatchImportModal isOpen={showImportModal} onClose={() => setShowImportModal(false)} onImport={handleImport} existingReagents={reagents} existingBatches={allBatches} />}
    </div>
  );
};

export default Reagents;