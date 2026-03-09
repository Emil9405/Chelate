// src/components/storage/LocationPicker.js
// Каскадный выбор позиции хранения: Комната → Зона → Позиция
// Используется в формах создания/редактирования батчей и оборудования

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { api } from '../../services/api';

// Иконки для типов зон
const ZONE_ICONS = {
  cabinet: 'fa-box',
  refrigerator: 'fa-temperature-low',
  freezer: 'fa-snowflake',
  fume_hood: 'fa-wind',
  safety_cabinet: 'fa-shield-alt',
  desiccator: 'fa-tint-slash',
  shelf: 'fa-layer-group',
  drawer: 'fa-th-list',
  other: 'fa-cube',
};

const ZONE_COLORS = {
  cabinet: '#6366f1',
  refrigerator: '#06b6d4',
  freezer: '#3b82f6',
  fume_hood: '#f59e0b',
  safety_cabinet: '#ef4444',
  desiccator: '#8b5cf6',
  shelf: '#10b981',
  drawer: '#f97316',
  other: '#6b7280',
};

const styles = {
  container: { position: 'relative' },
  label: { display: 'block', fontSize: '13px', fontWeight: '500', color: '#374151', marginBottom: '5px' },
  
  // Основное поле выбора - ИСПРАВЛЕНА ПРОБЛЕМА С BORDER
  selector: { 
    display: 'flex', 
    alignItems: 'center', 
    gap: '8px', 
    padding: '8px 12px', 
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#d1d5db',
    borderRadius: '8px', 
    cursor: 'pointer', 
    backgroundColor: 'white', 
    fontSize: '14px', 
    color: '#374151', 
    transition: 'border-color 0.15s, box-shadow 0.15s', 
    minHeight: '38px' 
  },
  selectorFocused: { borderColor: '#6366f1', boxShadow: '0 0 0 3px rgba(99,102,241,0.1)' },
  placeholder: { color: '#9ca3af', flex: 1 },
  selectedPath: { flex: 1, display: 'flex', alignItems: 'center', gap: '6px', fontSize: '13px' },
  pathSeparator: { color: '#d1d5db', fontSize: '10px' },
  pathSegment: { display: 'flex', alignItems: 'center', gap: '3px' },
  clearBtn: { width: '18px', height: '18px', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '50%', border: 'none', backgroundColor: '#f3f4f6', color: '#6b7280', cursor: 'pointer', fontSize: '10px', flexShrink: 0 },

  // Dropdown
  dropdown: { position: 'absolute', top: '100%', left: 0, right: 0, marginTop: '4px', backgroundColor: 'white', border: '1px solid #e5e7eb', borderRadius: '8px', boxShadow: '0 10px 25px -3px rgba(0,0,0,0.12)', zIndex: 50, maxHeight: '320px', overflow: 'auto' },
  
  // Search
  searchInput: { width: '100%', padding: '10px 12px', border: 'none', borderBottom: '1px solid #f3f4f6', fontSize: '13px', outline: 'none', boxSizing: 'border-box' },

  // Breadcrumb навигация
  breadcrumb: { display: 'flex', alignItems: 'center', gap: '6px', padding: '8px 12px', backgroundColor: '#f9fafb', borderBottom: '1px solid #f3f4f6', fontSize: '12px', color: '#6b7280' },
  breadcrumbBtn: { background: 'none', border: 'none', color: '#6366f1', cursor: 'pointer', padding: '2px 4px', fontSize: '12px', borderRadius: '4px' },
  breadcrumbCurrent: { color: '#374151', fontWeight: '500' },

  // Items в dropdown
  item: { display: 'flex', alignItems: 'center', gap: '10px', padding: '10px 12px', cursor: 'pointer', fontSize: '13px', transition: 'background-color 0.1s', borderBottom: '1px solid #f9fafb' },
  itemIcon: { width: '28px', height: '28px', borderRadius: '6px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'white', fontSize: '12px', flexShrink: 0 },
  itemInfo: { flex: 1, minWidth: 0 },
  itemName: { fontWeight: '500', color: '#374151' },
  itemMeta: { fontSize: '11px', color: '#9ca3af', marginTop: '1px' },
  itemArrow: { color: '#d1d5db', fontSize: '12px' },

  // Empty
  emptyDropdown: { padding: '20px', textAlign: 'center', color: '#9ca3af', fontSize: '13px' },
};

// ==================== КОМПОНЕНТ ====================

const LocationPicker = ({
  value,          // ID выбранной позиции (storage_position_id)
  onChange,        // (positionId: string | null) => void
  label = 'Storage Location',
  showLabel = true,
  required = false,
  disabled = false,
  placeholder = 'Select storage location...',
}) => {
  const [open, setOpen] = useState(false);
  const [hierarchy, setHierarchy] = useState([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [searchResults, setSearchResults] = useState(null);
  const containerRef = useRef(null);

  // Навигация: 'rooms' | { type: 'room', id, name } | { type: 'zone', id, name, roomName }
  const [level, setLevel] = useState('rooms');

  // Выбранный путь (для отображения)
  const [selectedPath, setSelectedPath] = useState(null);

  // Загрузка иерархии
  const loadHierarchy = useCallback(async () => {
    if (hierarchy.length > 0) return;
    setLoading(true);
    try {
      const data = await api.getStorageHierarchy();
      setHierarchy(data || []);
    } catch (err) {
      console.error('Failed to load hierarchy:', err);
    } finally {
      setLoading(false);
    }
  }, [hierarchy.length]);

  // Загрузить path для текущего value
  useEffect(() => {
    if (!value) {
      setSelectedPath(null);
      return;
    }
    const loadPath = async () => {
      try {
        const path = await api.getLocationPath(value);
        setSelectedPath(path);
      } catch {
        setSelectedPath(null);
      }
    };
    loadPath();
  }, [value]);

  // Закрытие при клике вне
  useEffect(() => {
    const handler = (e) => {
      if (containerRef.current && !containerRef.current.contains(e.target)) {
        setOpen(false);
        setSearch('');
        setSearchResults(null);
        setLevel('rooms');
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  // Поиск с debounce
  useEffect(() => {
    if (!search.trim()) {
      setSearchResults(null);
      return;
    }
    const timer = setTimeout(async () => {
      try {
        const results = await api.searchStorageLocations(search.trim());
        setSearchResults(results || []);
      } catch {
        setSearchResults([]);
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [search]);

  const handleOpen = () => {
    if (disabled) return;
    setOpen(true);
    setLevel('rooms');
    loadHierarchy();
  };

  const handleSelect = (positionId) => {
    onChange(positionId);
    setOpen(false);
    setSearch('');
    setSearchResults(null);
    setLevel('rooms');
  };

  const handleClear = (e) => {
    e.stopPropagation();
    onChange(null);
    setSelectedPath(null);
  };

  // Текущий уровень данных
  const currentItems = (() => {
    if (searchResults !== null) return null; // Показываем результаты поиска

    if (level === 'rooms') {
      return hierarchy.map(r => ({
        id: r.id,
        name: r.name,
        icon: 'fa-door-open',
        color: r.color || '#667eea',
        meta: `${r.zones?.length || 0} zones · ${r.total_items || 0} items`,
        hasChildren: (r.zones?.length || 0) > 0,
        type: 'room',
      }));
    }

    if (level?.type === 'room') {
      const room = hierarchy.find(r => r.id === level.id);
      if (!room) return [];
      return (room.zones || []).map(z => ({
        id: z.id,
        name: z.name,
        icon: ZONE_ICONS[z.zone_type] || 'fa-cube',
        color: ZONE_COLORS[z.zone_type] || '#6b7280',
        meta: `${z.positions?.length || 0} positions · ${z.items_count || 0} items`,
        hasChildren: (z.positions?.length || 0) > 0,
        type: 'zone',
        roomName: room.name,
      }));
    }

    if (level?.type === 'zone') {
      const room = hierarchy.find(r => r.zones?.some(z => z.id === level.id));
      if (!room) return [];
      const zoneData = room.zones?.find(z => z.id === level.id);
      if (!zoneData) return [];
      return (zoneData.positions || []).map(p => ({
        id: p.id,
        name: p.name,
        label: p.position_label,
        icon: 'fa-bookmark',
        color: '#10b981',
        meta: `${p.current_count} items${p.max_capacity ? ` / ${p.max_capacity} max` : ''}`,
        hasChildren: false,
        type: 'position',
      }));
    }

    return [];
  })();

  return (
    <div style={styles.container} ref={containerRef}>
      {showLabel && (
        <label style={styles.label}>
          {label} {required && <span style={{ color: '#ef4444' }}>*</span>}
        </label>
      )}

      {/* Selector field */}
      <div
        style={{
          ...styles.selector,
          ...(open ? styles.selectorFocused : {}),
          opacity: disabled ? 0.6 : 1,
          cursor: disabled ? 'not-allowed' : 'pointer',
        }}
        onClick={handleOpen}
      >
        {selectedPath ? (
          <div style={styles.selectedPath}>
            <span style={styles.pathSegment}>
              <i className="fas fa-door-open" style={{ fontSize: '11px', color: '#6366f1' }} />
              {selectedPath.room_name}
            </span>
            {selectedPath.zone_name && (
              <>
                <i className="fas fa-chevron-right" style={styles.pathSeparator} />
                <span style={styles.pathSegment}>
                  <i className={`fas ${ZONE_ICONS[selectedPath.zone_type] || 'fa-cube'}`}
                     style={{ fontSize: '11px', color: ZONE_COLORS[selectedPath.zone_type] || '#6b7280' }} />
                  {selectedPath.zone_name}
                </span>
              </>
            )}
            {selectedPath.position_name && (
              <>
                <i className="fas fa-chevron-right" style={styles.pathSeparator} />
                <span style={styles.pathSegment}>
                  <i className="fas fa-bookmark" style={{ fontSize: '11px', color: '#10b981' }} />
                  {selectedPath.position_name}
                  {selectedPath.position_label && (
                    <span style={{ color: '#9ca3af', fontSize: '11px' }}> ({selectedPath.position_label})</span>
                  )}
                </span>
              </>
            )}
          </div>
        ) : (
          <span style={styles.placeholder}>{placeholder}</span>
        )}

        {value && !disabled && (
          <button style={styles.clearBtn} onClick={handleClear} title="Clear">
            <i className="fas fa-times" />
          </button>
        )}
        <i className="fas fa-map-marker-alt" style={{ color: '#9ca3af', fontSize: '12px' }} />
      </div>

      {/* Dropdown */}
      {open && (
        <div style={styles.dropdown}>
          {/* Search */}
          <input
            style={styles.searchInput}
            placeholder="Search all locations..."
            value={search}
            onChange={e => setSearch(e.target.value)}
            autoFocus
          />

          {/* Breadcrumb */}
          {!searchResults && level !== 'rooms' && (
            <div style={styles.breadcrumb}>
              <button
                style={styles.breadcrumbBtn}
                onClick={() => setLevel('rooms')}
              >
                Rooms
              </button>
              {level?.type === 'room' && (
                <>
                  <i className="fas fa-chevron-right" style={{ fontSize: '8px' }} />
                  <span style={styles.breadcrumbCurrent}>{level.name}</span>
                </>
              )}
              {level?.type === 'zone' && (
                <>
                  <i className="fas fa-chevron-right" style={{ fontSize: '8px' }} />
                  <button
                    style={styles.breadcrumbBtn}
                    onClick={() => {
                      const room = hierarchy.find(r => r.zones?.some(z => z.id === level.id));
                      if (room) setLevel({ type: 'room', id: room.id, name: room.name });
                    }}
                  >
                    {level.roomName}
                  </button>
                  <i className="fas fa-chevron-right" style={{ fontSize: '8px' }} />
                  <span style={styles.breadcrumbCurrent}>{level.name}</span>
                </>
              )}
            </div>
          )}

          {/* Search results */}
          {searchResults !== null ? (
            searchResults.length === 0 ? (
              <div style={styles.emptyDropdown}>No locations found</div>
            ) : (
              searchResults.map(r => (
                <div
                  key={r.position_id}
                  style={styles.item}
                  onClick={() => handleSelect(r.position_id)}
                  onMouseEnter={e => e.currentTarget.style.backgroundColor = '#f3f4f6'}
                  onMouseLeave={e => e.currentTarget.style.backgroundColor = 'transparent'}
                >
                  <div style={{ ...styles.itemIcon, backgroundColor: ZONE_COLORS[r.zone_type] || '#6b7280' }}>
                    <i className={`fas ${ZONE_ICONS[r.zone_type] || 'fa-cube'}`} />
                  </div>
                  <div style={styles.itemInfo}>
                    <div style={styles.itemName}>{r.position_name}</div>
                    <div style={styles.itemMeta}>{r.full_path}</div>
                  </div>
                </div>
              ))
            )
          ) : loading ? (
            <div style={styles.emptyDropdown}>
              <i className="fas fa-spinner fa-spin" /> Loading...
            </div>
          ) : currentItems?.length === 0 ? (
            <div style={styles.emptyDropdown}>
              {level === 'rooms' ? 'No rooms available. Create rooms first.' : 'No items at this level.'}
            </div>
          ) : (
            currentItems?.map(item => (
              <div
                key={item.id}
                style={styles.item}
                onClick={() => {
                  if (item.type === 'position') {
                    handleSelect(item.id);
                  } else if (item.type === 'room') {
                    setLevel({ type: 'room', id: item.id, name: item.name });
                  } else if (item.type === 'zone') {
                    setLevel({ type: 'zone', id: item.id, name: item.name, roomName: item.roomName });
                  }
                }}
                onMouseEnter={e => e.currentTarget.style.backgroundColor = '#f3f4f6'}
                onMouseLeave={e => e.currentTarget.style.backgroundColor = 'transparent'}
              >
                <div style={{ ...styles.itemIcon, backgroundColor: item.color }}>
                  <i className={`fas ${item.icon}`} />
                </div>
                <div style={styles.itemInfo}>
                  <div style={styles.itemName}>
                    {item.name}
                    {item.label && <span style={{ color: '#9ca3af', fontWeight: '400' }}> ({item.label})</span>}
                  </div>
                  <div style={styles.itemMeta}>{item.meta}</div>
                </div>
                {item.hasChildren && (
                  <i className="fas fa-chevron-right" style={styles.itemArrow} />
                )}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};

export default LocationPicker;