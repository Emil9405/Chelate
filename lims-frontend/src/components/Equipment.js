// components/Equipment.js - Equipment management page
// Version 3.4 - Modals moved to EquipmentModals.js, fixed file_type to 'photo'
import React, { useState, useEffect, useCallback } from 'react';
import { api } from '../services/api';
import ErrorMessage from './ErrorMessage';
import Loading from './Loading';
import Badge from './Badge';
import Button from './Button';

// Import from EquipmentModals
import { 
  CreateEquipmentModal, 
  EditEquipmentModal, 
  EquipmentDetailsModal,
  PartFormModal,
  HoverImage,
  getFileUrl,
  PART_STATUSES,
  EQUIPMENT_STATUSES
} from './Modals/EquipmentModals';

// ==================== CONSTANTS ====================

const EQUIPMENT_TYPES = ['instrument', 'glassware', 'safety', 'storage', 'consumable', 'other'];

const inputStyle = {
  width: '100%', padding: '0.625rem 0.75rem', border: '1px solid #d1d5db', borderRadius: '6px',
  fontSize: '0.875rem', color: '#1f2937', backgroundColor: '#fff', outline: 'none', boxSizing: 'border-box'
};

// ==================== MAIN EQUIPMENT COMPONENT ====================

const Equipment = ({ user }) => {
  const [equipment, setEquipment] = useState([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [typeFilter, setTypeFilter] = useState('');
  const [allParts, setAllParts] = useState({});
  const [allFiles, setAllFiles] = useState({});
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showDetailsModal, setShowDetailsModal] = useState(false);
  const [selectedEquipment, setSelectedEquipment] = useState(null);
  const [editingEquipment, setEditingEquipment] = useState(null);
  const [error, setError] = useState('');
  const [expandedRows, setExpandedRows] = useState({});

  // Load equipment
  const loadEquipment = useCallback(async () => {
    try {
      setLoading(true);
      setError('');
      const response = await api.getEquipment({});
      let items = Array.isArray(response) ? response : (response?.data || []);
      setEquipment(items);

      // Load parts and files
      const partsMap = {}, filesMap = {};
      await Promise.all(items.map(async item => {
        try {
          const [parts, files] = await Promise.all([
            api.getEquipmentParts(item.id),
            api.getEquipmentFiles(item.id)
          ]);
          partsMap[item.id] = Array.isArray(parts) ? parts : [];
          filesMap[item.id] = Array.isArray(files) ? files : [];
        } catch {
          partsMap[item.id] = [];
          filesMap[item.id] = [];
        }
      }));
      setAllParts(partsMap);
      setAllFiles(filesMap);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadEquipment(); }, [loadEquipment]);

  const refreshEquipmentData = async (equipmentId) => {
    try {
      const [parts, files] = await Promise.all([
        api.getEquipmentParts(equipmentId),
        api.getEquipmentFiles(equipmentId)
      ]);
      setAllParts(prev => ({ ...prev, [equipmentId]: Array.isArray(parts) ? parts : [] }));
      setAllFiles(prev => ({ ...prev, [equipmentId]: Array.isArray(files) ? files : [] }));
    } catch (err) { console.error(err); }
  };

  const handleDelete = async (item) => {
    if (!window.confirm(`Delete "${item.name}"?`)) return;
    try {
      await api.deleteEquipment(item.id);
      loadEquipment();
    } catch (err) {
      setError(err.message);
    }
  };

  const toggleExpand = (id) => setExpandedRows(prev => ({ ...prev, [id]: !prev[id] }));

  const getEquipmentImage = (equipmentId) => {
    const files = allFiles[equipmentId] || [];
    const img = files.find(f => f.file_type === 'photo' && !f.part_id);
    return img ? getFileUrl(equipmentId, img.id) : null;
  };

  // Global search (equipment + parts)
  const filteredEquipment = equipment.filter(item => {
    if (!item) return false;
    const matchesStatus = !statusFilter || item.status === statusFilter;
    const matchesType = !typeFilter || item.type_ === typeFilter;
    if (!matchesStatus || !matchesType) return false;
    if (!searchTerm) return true;
    
    const term = searchTerm.toLowerCase();
    const matchesEquipment = 
      item.name?.toLowerCase().includes(term) ||
      item.serial_number?.toLowerCase().includes(term) ||
      item.manufacturer?.toLowerCase().includes(term);
    if (matchesEquipment) return true;
    
    const parts = allParts[item.id] || [];
    return parts.some(part => 
      part.name?.toLowerCase().includes(term) ||
      part.part_number?.toLowerCase().includes(term)
    );
  });

  const getMatchingPartsCount = (equipmentId) => {
    if (!searchTerm) return 0;
    const term = searchTerm.toLowerCase();
    const parts = allParts[equipmentId] || [];
    return parts.filter(p => 
      p.name?.toLowerCase().includes(term) || 
      p.part_number?.toLowerCase().includes(term)
    ).length;
  };

  const canEdit = () => ['Admin', 'Researcher'].includes(user?.role);
  const equipmentTypes = [...new Set(equipment.map(e => e.type_).filter(Boolean))];

  return (
    <div style={{ padding: '6rem 2rem 2rem 2rem' }}>
      {/* Page Header */}
      <div style={{ marginBottom: '2rem' }}>
        <h1 style={{ fontSize: '2rem', fontWeight: '600', color: '#2d3748', marginBottom: '0.5rem' }}>
          <i className="fas fa-tools" style={{ marginRight: '0.75rem', color: '#667eea' }}></i>
          Equipment Management
        </h1>
        <p style={{ color: '#718096' }}>Manage laboratory equipment, parts, and maintenance</p>
      </div>

      {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}

      {/* Main Card */}
      <div style={{ background: 'white', borderRadius: '12px', boxShadow: '0 4px 20px rgba(0,0,0,0.05)', overflow: 'hidden' }}>
        {/* Card Header */}
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0', display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
          <div>
            <h2 style={{ fontSize: '1.25rem', fontWeight: '600', color: '#2d3748', margin: 0 }}>Equipment List</h2>
            <p style={{ fontSize: '0.875rem', color: '#718096', margin: '0.25rem 0 0 0' }}>Total: {filteredEquipment.length} items</p>
          </div>
          {canEdit() && (
            <Button variant="primary" onClick={() => setShowCreateModal(true)}>
              <i className="fas fa-plus"></i> Add Equipment
            </Button>
          )}
        </div>

        {/* Filters */}
        <div style={{ padding: '1.5rem 2rem', borderBottom: '1px solid #e2e8f0', display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'flex-end' }}>
          <div style={{ flex: 1, minWidth: '300px' }}>
            <label style={{ display: 'block', fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>
              Search equipment & parts (name, S/N, P/N)
            </label>
            <input placeholder="Search..." value={searchTerm} onChange={(e) => setSearchTerm(e.target.value)} style={inputStyle} />
          </div>
          <div style={{ width: '180px' }}>
            <select value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)} style={{ ...inputStyle, cursor: 'pointer' }}>
              <option value="">All Types</option>
              {equipmentTypes.map(type => <option key={type} value={type}>{type}</option>)}
            </select>
          </div>
          <div style={{ width: '180px' }}>
            <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} style={{ ...inputStyle, cursor: 'pointer' }}>
              <option value="">All Statuses</option>
              {Object.entries(EQUIPMENT_STATUSES).map(([value, { label }]) => <option key={value} value={value}>{label}</option>)}
            </select>
          </div>
        </div>

        {/* Equipment Table */}
        <div style={{ padding: '0' }}>
          {loading ? <div style={{ padding: '2rem' }}><Loading /></div> : filteredEquipment.length > 0 ? (
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ background: '#f8fafc', borderBottom: '2px solid #e2e8f0' }}>
                  <th style={{ width: '40px', padding: '1rem' }}></th>
                  <th style={{ width: '60px', padding: '1rem' }}></th>
                  <th style={{ textAlign: 'left', padding: '1rem', color: '#4a5568', fontWeight: '600' }}>Name</th>
                  <th style={{ textAlign: 'left', padding: '1rem', color: '#4a5568', fontWeight: '600' }}>Type</th>
                  <th style={{ textAlign: 'left', padding: '1rem', color: '#4a5568', fontWeight: '600' }}>S/N</th>
                  <th style={{ textAlign: 'left', padding: '1rem', color: '#4a5568', fontWeight: '600' }}>Location</th>
                  <th style={{ textAlign: 'left', padding: '1rem', color: '#4a5568', fontWeight: '600' }}>Status</th>
                  <th style={{ textAlign: 'right', padding: '1rem', color: '#4a5568', fontWeight: '600' }}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredEquipment.map(item => {
                  const matchingParts = getMatchingPartsCount(item.id);
                  return (
                    <React.Fragment key={item.id}>
                      <tr 
                        style={{ borderBottom: expandedRows[item.id] ? 'none' : '1px solid #e2e8f0', background: expandedRows[item.id] ? '#f0f4ff' : 'white', cursor: 'pointer' }}
                        onMouseEnter={(e) => { if (!expandedRows[item.id]) e.currentTarget.style.background = '#f8fafc'; }}
                        onMouseLeave={(e) => { if (!expandedRows[item.id]) e.currentTarget.style.background = 'white'; }}
                      >
                        <td style={{ padding: '1rem', textAlign: 'center' }} onClick={() => toggleExpand(item.id)}>
                          <i className={`fas fa-chevron-${expandedRows[item.id] ? 'down' : 'right'}`} style={{ color: '#667eea', fontSize: '0.875rem' }}></i>
                        </td>
                        <td style={{ padding: '0.5rem' }} onClick={() => toggleExpand(item.id)}>
                          <HoverImage src={getEquipmentImage(item.id)} alt={item.name} size={44} zoomSize={250} />
                        </td>
                        <td style={{ padding: '1rem' }} onClick={() => toggleExpand(item.id)}>
                          <div>
                            <div style={{ fontWeight: '500', color: '#2d3748' }}>{item.name}</div>
                            {item.manufacturer && <div style={{ fontSize: '0.75rem', color: '#718096' }}>{item.manufacturer}</div>}
                            {matchingParts > 0 && (
                              <div style={{ fontSize: '0.7rem', color: '#667eea', marginTop: '0.25rem' }}>
                                <i className="fas fa-cogs"></i> {matchingParts} matching part{matchingParts > 1 ? 's' : ''}
                              </div>
                            )}
                          </div>
                        </td>
                        <td style={{ padding: '1rem', color: '#4a5568' }} onClick={() => toggleExpand(item.id)}>{item.type_}</td>
                        <td style={{ padding: '1rem', color: '#718096', fontFamily: 'monospace', fontSize: '0.875rem' }} onClick={() => toggleExpand(item.id)}>{item.serial_number || '-'}</td>
                        <td style={{ padding: '1rem', color: '#4a5568' }} onClick={() => toggleExpand(item.id)}>{item.location || '-'}</td>
                        <td style={{ padding: '1rem' }} onClick={() => toggleExpand(item.id)}>
                          <Badge variant={EQUIPMENT_STATUSES[item.status]?.color || 'secondary'}>{EQUIPMENT_STATUSES[item.status]?.label || item.status}</Badge>
                        </td>
                        <td style={{ padding: '1rem', textAlign: 'right' }}>
                          <Button variant="secondary" size="small" onClick={() => { setSelectedEquipment(item); setShowDetailsModal(true); }} style={{ marginRight: '0.5rem' }} title="View Details">
                            <i className="fas fa-eye"></i>
                          </Button>
                          {canEdit() && (
                            <>
                              <Button variant="primary" size="small" onClick={() => setEditingEquipment(item)} style={{ marginRight: '0.5rem' }} title="Edit">
                                <i className="fas fa-edit"></i>
                              </Button>
                              <Button variant="danger" size="small" onClick={() => handleDelete(item)} title="Delete">
                                <i className="fas fa-trash"></i>
                              </Button>
                            </>
                          )}
                        </td>
                      </tr>
                      {expandedRows[item.id] && (
                        <tr>
                          <td colSpan="8" style={{ padding: 0, background: '#f8fafc', borderBottom: '1px solid #e2e8f0' }}>
                            <ExpandedPartsPanel 
                              equipmentId={item.id} 
                              parts={allParts[item.id] || []} 
                              files={allFiles[item.id] || []}
                              searchTerm={searchTerm}
                              canEdit={canEdit()} 
                              onRefresh={() => refreshEquipmentData(item.id)} 
                            />
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  );
                })}
              </tbody>
            </table>
          ) : (
            <div style={{ textAlign: 'center', padding: '3rem', color: '#a0aec0' }}>
              <i className="fas fa-tools" style={{ fontSize: '3rem', marginBottom: '1rem' }}></i>
              <p>No equipment found</p>
            </div>
          )}
        </div>
      </div>

      {/* Modals */}
      {showCreateModal && (
        <CreateEquipmentModal 
          isOpen={showCreateModal}
          onClose={() => setShowCreateModal(false)} 
          onSave={() => { setShowCreateModal(false); loadEquipment(); }} 
        />
      )}
      {editingEquipment && (
        <EditEquipmentModal 
          isOpen={!!editingEquipment}
          equipment={editingEquipment} 
          existingImage={getEquipmentImage(editingEquipment.id)}
          onClose={() => setEditingEquipment(null)} 
          onSave={() => { setEditingEquipment(null); loadEquipment(); }} 
        />
      )}
      {showDetailsModal && selectedEquipment && (
        <EquipmentDetailsModal 
          equipment={selectedEquipment} 
          user={user} 
          onClose={() => { setShowDetailsModal(false); setSelectedEquipment(null); }}
          onUpdate={() => { loadEquipment(); refreshEquipmentData(selectedEquipment.id); }} 
        />
      )}
    </div>
  );
};

// ==================== EXPANDED PARTS PANEL ====================

const ExpandedPartsPanel = ({ equipmentId, parts, files, searchTerm, canEdit, onRefresh }) => {
  const [showAddModal, setShowAddModal] = useState(false);

  const handleDelete = async (partId) => {
    if (window.confirm('Delete this part?')) {
      try {
        await api.deleteEquipmentPart(equipmentId, partId);
        onRefresh();
      } catch (err) { console.error(err); }
    }
  };

  const getPartImage = (partId) => {
    const img = files.find(f => f.file_type === 'photo' && f.part_id === partId);
    return img ? getFileUrl(equipmentId, img.id) : null;
  };

  // Highlight matching parts
  const isPartMatching = (part) => {
    if (!searchTerm) return false;
    const term = searchTerm.toLowerCase();
    return part.name?.toLowerCase().includes(term) || part.part_number?.toLowerCase().includes(term);
  };

  return (
    <div style={{ padding: '1rem 1rem 1rem 4rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.75rem' }}>
        <h4 style={{ margin: 0, fontSize: '0.875rem', color: '#4a5568', fontWeight: '600' }}>
          <i className="fas fa-cogs" style={{ marginRight: '0.5rem', color: '#667eea' }}></i>Parts ({parts.length})
        </h4>
        {canEdit && <Button variant="primary" size="small" onClick={() => setShowAddModal(true)}><i className="fas fa-plus"></i> Add Part</Button>}
      </div>
      {parts.length === 0 ? (
        <p style={{ color: '#a0aec0', fontSize: '0.875rem', margin: '0.5rem 0' }}>No parts registered</p>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.875rem' }}>
          <thead>
            <tr style={{ background: '#edf2f7' }}>
              <th style={{ width: '50px', padding: '0.5rem' }}></th>
              <th style={{ textAlign: 'left', padding: '0.5rem', color: '#4a5568' }}>Name</th>
              <th style={{ textAlign: 'left', padding: '0.5rem', color: '#4a5568' }}>P/N</th>
              <th style={{ textAlign: 'left', padding: '0.5rem', color: '#4a5568' }}>Manufacturer</th>
              <th style={{ textAlign: 'center', padding: '0.5rem', color: '#4a5568' }}>Qty</th>
              <th style={{ textAlign: 'left', padding: '0.5rem', color: '#4a5568' }}>Status</th>
              {canEdit && <th style={{ textAlign: 'right', padding: '0.5rem' }}>Actions</th>}
            </tr>
          </thead>
          <tbody>
            {parts.map(part => (
              <tr 
                key={part.id} 
                style={{ 
                  borderBottom: '1px solid #e2e8f0',
                  background: isPartMatching(part) ? '#fef3c7' : 'transparent'
                }}
              >
                <td style={{ padding: '0.5rem' }}>
                  <HoverImage src={getPartImage(part.id)} alt={part.name} size={36} zoomSize={180} />
                </td>
                <td style={{ padding: '0.5rem', fontWeight: '500' }}>{part.name}</td>
                <td style={{ padding: '0.5rem', color: '#718096', fontFamily: 'monospace' }}>{part.part_number || '-'}</td>
                <td style={{ padding: '0.5rem', color: '#718096' }}>{part.manufacturer || '-'}</td>
                <td style={{ padding: '0.5rem', textAlign: 'center' }}>{part.quantity}</td>
                <td style={{ padding: '0.5rem' }}>
                  <Badge variant={PART_STATUSES[part.status]?.color || 'secondary'}>
                    {PART_STATUSES[part.status]?.label || part.status}
                  </Badge>
                </td>
                {canEdit && (
                  <td style={{ padding: '0.5rem', textAlign: 'right' }}>
                    <button
                      onClick={(e) => { e.stopPropagation(); handleDelete(part.id); }}
                      style={{ background: 'none', border: 'none', color: '#e53e3e', cursor: 'pointer', padding: '0.25rem' }}
                    >
                      <i className="fas fa-trash"></i>
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {showAddModal && (
        <PartFormModal
          equipmentId={equipmentId}
          onClose={() => setShowAddModal(false)}
          onSave={() => { setShowAddModal(false); onRefresh(); }}
        />
      )}
    </div>
  );
};

export default Equipment;