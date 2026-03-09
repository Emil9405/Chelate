// src/components/storage/LocationManager.js
// Полноценный модуль управления локациями хранения
// Иерархия: Комната → Зона хранения (шкаф/холодильник) → Позиция (полка/ящик)

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { api } from '../../services/api';

// ==================== КОНСТАНТЫ ====================

const ZONE_TYPES = [
  { value: 'cabinet', label: 'Cabinet', icon: 'fa-box', color: '#6366f1' },
  { value: 'refrigerator', label: 'Refrigerator', icon: 'fa-temperature-low', color: '#06b6d4' },
  { value: 'freezer', label: 'Freezer', icon: 'fa-snowflake', color: '#3b82f6' },
  { value: 'fume_hood', label: 'Fume Hood', icon: 'fa-wind', color: '#f59e0b' },
  { value: 'safety_cabinet', label: 'Safety Cabinet', icon: 'fa-shield-alt', color: '#ef4444' },
  { value: 'desiccator', label: 'Desiccator', icon: 'fa-tint-slash', color: '#8b5cf6' },
  { value: 'shelf', label: 'Shelf Unit', icon: 'fa-layer-group', color: '#10b981' },
  { value: 'drawer', label: 'Drawer Unit', icon: 'fa-th-list', color: '#f97316' },
  { value: 'other', label: 'Other', icon: 'fa-cube', color: '#6b7280' },
];

const STORAGE_CONDITIONS = [
  { value: 'room_temperature', label: 'Room Temperature (15–25°C)' },
  { value: 'cool', label: 'Cool (+2…+8°C)' },
  { value: 'frozen', label: 'Frozen (−20°C)' },
  { value: 'deep_frozen', label: 'Deep Frozen (−80°C)' },
  { value: 'ventilated', label: 'Ventilated' },
  { value: 'dry_storage', label: 'Dry Storage' },
  { value: 'light_protected', label: 'Light Protected' },
];

const ROOM_TYPES = [
  { value: 'general', label: 'General' },
  { value: 'wet_lab', label: 'Wet Lab' },
  { value: 'dry_lab', label: 'Dry Lab' },
  { value: 'storage_room', label: 'Storage Room' },
  { value: 'instrument_room', label: 'Instrument Room' },
  { value: 'prep_room', label: 'Prep Room' },
  { value: 'cold_room', label: 'Cold Room' },
];

const getZoneTypeInfo = (type_) => ZONE_TYPES.find(z => z.value === type_) || ZONE_TYPES[ZONE_TYPES.length - 1];

// ==================== СТИЛИ ====================

const S = {
  // Layout
  container: { padding: '100px 24px 24px', maxWidth: '1200px', margin: '0 auto' },
  header: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' },
  title: { fontSize: '22px', fontWeight: '600', color: '#1f2937', display: 'flex', alignItems: 'center', gap: '10px' },
  subtitle: { fontSize: '13px', color: '#6b7280', fontWeight: '400', marginTop: '2px' },
  
  // Toolbar
  toolbar: { display: 'flex', gap: '10px', alignItems: 'center', flexWrap: 'wrap' },
  searchBox: { padding: '8px 14px', border: '1px solid #e5e7eb', borderRadius: '8px', fontSize: '14px', width: '250px', outline: 'none', transition: 'border-color 0.15s, box-shadow 0.15s' },
  addBtn: { padding: '8px 16px', borderRadius: '8px', border: 'none', cursor: 'pointer', fontWeight: '500', fontSize: '13px', display: 'flex', alignItems: 'center', gap: '6px', transition: 'all 0.15s', color: 'white', backgroundColor: '#6366f1' },
  
  // Room Card
  roomCard: { backgroundColor: '#fff', border: '1px solid #e5e7eb', borderRadius: '12px', marginBottom: '16px', overflow: 'hidden', transition: 'box-shadow 0.15s' },
  roomHeader: { display: 'flex', alignItems: 'center', gap: '12px', padding: '16px 20px', cursor: 'pointer', userSelect: 'none', transition: 'background-color 0.1s' },
  roomColorBar: { width: '4px', height: '40px', borderRadius: '2px', flexShrink: 0 },
  roomName: { flex: 1, fontWeight: '600', fontSize: '16px', color: '#1f2937' },
  roomMeta: { display: 'flex', gap: '16px', fontSize: '12px', color: '#6b7280' },
  roomBadge: { padding: '2px 8px', borderRadius: '10px', fontSize: '11px', fontWeight: '500' },
  chevron: { color: '#9ca3af', transition: 'transform 0.2s', fontSize: '14px' },
  
  // Zone
  zonesContainer: { padding: '0 20px 16px', borderTop: '1px solid #f3f4f6' },
  zoneCard: { backgroundColor: '#f9fafb', border: '1px solid #e5e7eb', borderRadius: '8px', marginTop: '12px', overflow: 'hidden' },
  zoneHeader: { display: 'flex', alignItems: 'center', gap: '10px', padding: '12px 16px', cursor: 'pointer', userSelect: 'none' },
  zoneIcon: { width: '32px', height: '32px', borderRadius: '6px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'white', fontSize: '14px', flexShrink: 0 },
  zoneName: { flex: 1, fontWeight: '500', fontSize: '14px', color: '#374151' },
  zoneType: { fontSize: '11px', color: '#6b7280', fontWeight: '400' },
  zoneMeta: { fontSize: '11px', color: '#9ca3af' },
  
  // Position
  positionsGrid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))', gap: '8px', padding: '0 16px 12px' },
  positionCard: { padding: '10px 12px', backgroundColor: 'white', border: '1px solid #e5e7eb', borderRadius: '6px', fontSize: '13px', cursor: 'pointer', transition: 'all 0.15s', position: 'relative' },
  positionName: { fontWeight: '500', color: '#374151' },
  positionLabel: { fontSize: '11px', color: '#6b7280', marginTop: '2px' },
  positionCount: { fontSize: '11px', color: '#9ca3af', marginTop: '4px', display: 'flex', alignItems: 'center', gap: '4px' },
  capacityBar: { height: '3px', backgroundColor: '#e5e7eb', borderRadius: '2px', marginTop: '6px', overflow: 'hidden' },
  capacityFill: { height: '100%', borderRadius: '2px', transition: 'width 0.3s' },

  // Forms
  formOverlay: { position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0,0,0,0.4)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000 },
  formModal: { backgroundColor: 'white', borderRadius: '12px', padding: '24px', width: '480px', maxWidth: '90vw', maxHeight: '85vh', overflow: 'auto', boxShadow: '0 20px 25px -5px rgba(0,0,0,0.1)' },
  formTitle: { fontSize: '18px', fontWeight: '600', color: '#1f2937', marginBottom: '20px', display: 'flex', alignItems: 'center', gap: '8px' },
  formGroup: { marginBottom: '16px' },
  formLabel: { display: 'block', fontSize: '13px', fontWeight: '500', color: '#374151', marginBottom: '5px' },
  formInput: { width: '100%', padding: '9px 12px', border: '1px solid #d1d5db', borderRadius: '8px', fontSize: '14px', outline: 'none', boxSizing: 'border-box', transition: 'border-color 0.15s' },
  formSelect: { width: '100%', padding: '9px 12px', border: '1px solid #d1d5db', borderRadius: '8px', fontSize: '14px', outline: 'none', boxSizing: 'border-box', backgroundColor: 'white' },
  formRow: { display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' },
  formActions: { display: 'flex', gap: '10px', justifyContent: 'flex-end', marginTop: '24px' },
  btnPrimary: { padding: '9px 20px', backgroundColor: '#6366f1', color: 'white', border: 'none', borderRadius: '8px', cursor: 'pointer', fontWeight: '500', fontSize: '14px' },
  btnSecondary: { padding: '9px 20px', backgroundColor: '#f3f4f6', color: '#374151', border: '1px solid #d1d5db', borderRadius: '8px', cursor: 'pointer', fontWeight: '500', fontSize: '14px' },
  btnDanger: { padding: '6px 12px', backgroundColor: '#fee2e2', color: '#dc2626', border: 'none', borderRadius: '6px', cursor: 'pointer', fontSize: '12px' },
  btnSmall: { padding: '4px 10px', borderRadius: '6px', border: '1px solid #e5e7eb', backgroundColor: 'white', cursor: 'pointer', fontSize: '12px', color: '#374151', display: 'inline-flex', alignItems: 'center', gap: '4px' },
  
  // Add inline buttons
  addZoneBtn: { display: 'flex', alignItems: 'center', gap: '6px', padding: '8px 14px', backgroundColor: 'transparent', border: '1px dashed #d1d5db', borderRadius: '8px', color: '#6b7280', cursor: 'pointer', fontSize: '13px', margin: '12px 0 4px', width: '100%', justifyContent: 'center', transition: 'all 0.15s' },
  addPositionBtn: { display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px', padding: '8px', backgroundColor: 'transparent', border: '1px dashed #d1d5db', borderRadius: '6px', color: '#9ca3af', cursor: 'pointer', fontSize: '12px', minHeight: '60px', transition: 'all 0.15s' },

  // Empty state
  emptyState: { textAlign: 'center', padding: '60px 20px', color: '#6b7280' },
  emptyIcon: { fontSize: '48px', color: '#d1d5db', marginBottom: '16px' },
  emptyText: { fontSize: '15px', marginBottom: '8px' },
  emptyHint: { fontSize: '13px', color: '#9ca3af' },

  // Context menu
  actionMenu: { position: 'absolute', right: '8px', top: '8px', display: 'flex', gap: '4px', opacity: 0, transition: 'opacity 0.15s' },
  actionBtn: { width: '24px', height: '24px', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '4px', border: 'none', cursor: 'pointer', fontSize: '11px', backgroundColor: 'white', color: '#6b7280', boxShadow: '0 1px 2px rgba(0,0,0,0.1)' },

  // Items preview
  itemsPreview: { padding: '8px 16px 12px', borderTop: '1px solid #e5e7eb', backgroundColor: '#fefefe' },
  itemRow: { display: 'flex', alignItems: 'center', gap: '8px', padding: '4px 0', fontSize: '12px', color: '#6b7280' },

  // Stats bar
  statsBar: { display: 'flex', gap: '24px', padding: '12px 20px', backgroundColor: '#f9fafb', borderRadius: '8px', marginBottom: '20px', fontSize: '13px', color: '#6b7280' },
  statItem: { display: 'flex', alignItems: 'center', gap: '6px' },
  statValue: { fontWeight: '600', color: '#1f2937', fontSize: '16px' },

  // Loading
  loading: { textAlign: 'center', padding: '40px', color: '#9ca3af', fontSize: '14px' },
};

// ==================== ФОРМА КОМНАТЫ ====================

const RoomForm = ({ room, onSave, onCancel }) => {
  const isEdit = !!room;
  const [form, setForm] = useState({
    name: room?.name || '',
    description: room?.description || '',
    capacity: room?.capacity ?? '',
    color: room?.color || '#667eea',
  });
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!form.name.trim()) return alert('Room name is required');
    setSaving(true);
    try {
      const payload = {
        name: form.name.trim(),
        description: form.description.trim() || null,
        capacity: form.capacity !== '' ? parseInt(form.capacity) : null,
        color: form.color,
      };
      if (isEdit) {
        await api.updateRoom(room.id, payload);
      } else {
        await api.createRoom(payload);
      }
      onSave();
    } catch (err) {
      alert(err.message || 'Error saving room');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={S.formOverlay} onClick={onCancel}>
      <div style={S.formModal} onClick={e => e.stopPropagation()}>
        <div style={S.formTitle}>
          <i className="fas fa-door-open" style={{ color: '#6366f1' }} />
          {isEdit ? 'Edit Room' : 'Add Room'}
        </div>

        <div style={S.formGroup}>
          <label style={S.formLabel}>Room Name *</label>
          <input
            style={S.formInput}
            value={form.name}
            onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
            placeholder="e.g., Lab 104, Storage Room B"
            autoFocus
          />
        </div>

        <div style={S.formRow}>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Capacity (seats)</label>
            <input
              style={S.formInput}
              type="number"
              min="1"
              value={form.capacity}
              onChange={e => setForm(f => ({ ...f, capacity: e.target.value }))}
              placeholder="Optional"
            />
          </div>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Color</label>
            <input
              type="color"
              value={form.color}
              onChange={e => setForm(f => ({ ...f, color: e.target.value }))}
              style={{ width: '100%', height: '38px', padding: '2px', border: '1px solid #d1d5db', borderRadius: '8px', cursor: 'pointer' }}
            />
          </div>
        </div>

        <div style={S.formGroup}>
          <label style={S.formLabel}>Description</label>
          <input
            style={S.formInput}
            value={form.description}
            onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
            placeholder="Optional notes about this room"
          />
        </div>

        <div style={S.formActions}>
          <button style={S.btnSecondary} onClick={onCancel}>Cancel</button>
          <button style={S.btnPrimary} onClick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : (isEdit ? 'Update Room' : 'Add Room')}
          </button>
        </div>
      </div>
    </div>
  );
};

// ==================== ФОРМА СОЗДАНИЯ ЗОНЫ ====================

const ZoneForm = ({ roomId, zone, onSave, onCancel }) => {
  const isEdit = !!zone;
  const [form, setForm] = useState({
    name: zone?.name || '',
    zone_type: zone?.zone_type || 'cabinet',
    storage_condition: zone?.storage_condition || '',
    description: zone?.description || '',
    temperature_min: zone?.temperature_min ?? '',
    temperature_max: zone?.temperature_max ?? '',
    is_locked: zone?.is_locked === 1,
  });
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!form.name.trim()) return alert('Name is required');
    setSaving(true);
    try {
      const payload = {
        name: form.name.trim(),
        zone_type: form.zone_type,
        storage_condition: form.storage_condition || null,
        description: form.description.trim() || null,
        temperature_min: form.temperature_min !== '' ? parseFloat(form.temperature_min) : null,
        temperature_max: form.temperature_max !== '' ? parseFloat(form.temperature_max) : null,
        is_locked: form.is_locked,
      };
      if (!isEdit) payload.room_id = roomId;
      
      if (isEdit) {
        await api.updateStorageZone(zone.id, payload);
      } else {
        await api.createStorageZone(payload);
      }
      onSave();
    } catch (err) {
      alert(err.message || 'Error saving zone');
    } finally {
      setSaving(false);
    }
  };

  const typeInfo = getZoneTypeInfo(form.zone_type);

  return (
    <div style={S.formOverlay} onClick={onCancel}>
      <div style={S.formModal} onClick={e => e.stopPropagation()}>
        <div style={S.formTitle}>
          <i className={`fas ${typeInfo.icon}`} style={{ color: typeInfo.color }} />
          {isEdit ? 'Edit Storage Zone' : 'Add Storage Zone'}
        </div>
        
        <div style={S.formGroup}>
          <label style={S.formLabel}>Name *</label>
          <input
            style={S.formInput}
            value={form.name}
            onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
            placeholder="e.g., Cabinet A, Fridge-1"
            autoFocus
          />
        </div>

        <div style={S.formRow}>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Zone Type *</label>
            <select
              style={S.formSelect}
              value={form.zone_type}
              onChange={e => setForm(f => ({ ...f, zone_type: e.target.value }))}
            >
              {ZONE_TYPES.map(t => (
                <option key={t.value} value={t.value}>{t.label}</option>
              ))}
            </select>
          </div>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Storage Condition</label>
            <select
              style={S.formSelect}
              value={form.storage_condition}
              onChange={e => setForm(f => ({ ...f, storage_condition: e.target.value }))}
            >
              <option value="">— Not specified —</option>
              {STORAGE_CONDITIONS.map(c => (
                <option key={c.value} value={c.value}>{c.label}</option>
              ))}
            </select>
          </div>
        </div>

        <div style={S.formRow}>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Temp Min (°C)</label>
            <input
              style={S.formInput}
              type="number"
              value={form.temperature_min}
              onChange={e => setForm(f => ({ ...f, temperature_min: e.target.value }))}
              placeholder="e.g., 2"
            />
          </div>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Temp Max (°C)</label>
            <input
              style={S.formInput}
              type="number"
              value={form.temperature_max}
              onChange={e => setForm(f => ({ ...f, temperature_max: e.target.value }))}
              placeholder="e.g., 8"
            />
          </div>
        </div>

        <div style={S.formGroup}>
          <label style={S.formLabel}>Description</label>
          <input
            style={S.formInput}
            value={form.description}
            onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
            placeholder="Optional notes"
          />
        </div>

        <div style={S.formGroup}>
          <label style={{ ...S.formLabel, display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
            <input
              type="checkbox"
              checked={form.is_locked}
              onChange={e => setForm(f => ({ ...f, is_locked: e.target.checked }))}
            />
            Locked (key required)
          </label>
        </div>

        <div style={S.formActions}>
          <button style={S.btnSecondary} onClick={onCancel}>Cancel</button>
          <button style={S.btnPrimary} onClick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : (isEdit ? 'Update' : 'Add Zone')}
          </button>
        </div>
      </div>
    </div>
  );
};

// ==================== ФОРМА ПОЗИЦИИ ====================

const PositionForm = ({ zoneId, position, onSave, onCancel }) => {
  const isEdit = !!position;
  const [form, setForm] = useState({
    name: position?.name || '',
    position_label: position?.position_label || '',
    max_capacity: position?.max_capacity ?? '',
    description: position?.description || '',
  });
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!form.name.trim()) return alert('Name is required');
    setSaving(true);
    try {
      const payload = {
        name: form.name.trim(),
        position_label: form.position_label.trim() || null,
        max_capacity: form.max_capacity !== '' ? parseInt(form.max_capacity) : null,
        description: form.description.trim() || null,
      };
      if (!isEdit) payload.zone_id = zoneId;

      if (isEdit) {
        await api.updateStoragePosition(position.id, payload);
      } else {
        await api.createStoragePosition(payload);
      }
      onSave();
    } catch (err) {
      alert(err.message || 'Error saving position');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={S.formOverlay} onClick={onCancel}>
      <div style={{ ...S.formModal, width: '400px' }} onClick={e => e.stopPropagation()}>
        <div style={S.formTitle}>
          <i className="fas fa-bookmark" style={{ color: '#10b981' }} />
          {isEdit ? 'Edit Position' : 'Add Position'}
        </div>

        <div style={S.formGroup}>
          <label style={S.formLabel}>Name *</label>
          <input
            style={S.formInput}
            value={form.name}
            onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
            placeholder="e.g., Shelf 1, Drawer A"
            autoFocus
          />
        </div>

        <div style={S.formRow}>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Label / Code</label>
            <input
              style={S.formInput}
              value={form.position_label}
              onChange={e => setForm(f => ({ ...f, position_label: e.target.value }))}
              placeholder="e.g., A1, S3"
            />
          </div>
          <div style={S.formGroup}>
            <label style={S.formLabel}>Max Capacity</label>
            <input
              style={S.formInput}
              type="number"
              min="1"
              value={form.max_capacity}
              onChange={e => setForm(f => ({ ...f, max_capacity: e.target.value }))}
              placeholder="Unlimited"
            />
          </div>
        </div>

        <div style={S.formGroup}>
          <label style={S.formLabel}>Description</label>
          <input
            style={S.formInput}
            value={form.description}
            onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
            placeholder="Optional"
          />
        </div>

        <div style={S.formActions}>
          <button style={S.btnSecondary} onClick={onCancel}>Cancel</button>
          <button style={{ ...S.btnPrimary, backgroundColor: '#10b981' }} onClick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : (isEdit ? 'Update' : 'Add Position')}
          </button>
        </div>
      </div>
    </div>
  );
};

// ==================== ГРУППИРОВКА КОНТЕЙНЕРОВ ====================

const groupItems = (items) => {
  if (!items || items.length === 0) return [];

  const groups = new Map();

  for (const item of items) {
    // Теперь данные чистые: берем container_quantity напрямую
    const qty = item.container_quantity ?? 0;
    
    // Бэкенд теперь отдает честный boolean is_opened
    const isOpened = item.is_opened === true; 

    const displayStatus = isOpened ? 'Opened' : 'Sealed';
    const posKey = item.position_id || item.position_name || 'unplaced';

    // КЛЮЧЕВОЕ: Бэкенд теперь передает настоящий container_id!
    // Для открытых используем его, чтобы они не слипались.
    // Закрытые слипаются в одну строку.
    const uniqueId = item.container_id || Math.random();
    
    const key = isOpened
      ? `${item.batch_number}::${displayStatus}::${qty}::${posKey}::${uniqueId}`
      : `${item.batch_number}::${displayStatus}::${qty}::${posKey}`;

    if (groups.has(key)) {
      const g = groups.get(key);
      g.count += 1;
      g.total_quantity += qty;
    } else {
      groups.set(key, {
        ...item,
        computed_quantity: qty,
        computed_status: displayStatus,
        count: 1,
        total_quantity: qty,
        is_open: isOpened,
      });
    }
  }

  return Array.from(groups.values());
};

const fmtQty = (n) => typeof n === 'number' ? n.toLocaleString(undefined, { maximumFractionDigits: 2 }) : n;



// ==================== ТАБЛИЦА СОДЕРЖИМОГО (МОДАЛ) ====================

const ItemsTable = ({ items, loading, showPosition }) => {
  if (loading) {
    return <div style={{ padding: '16px', textAlign: 'center', color: '#9ca3af', fontSize: '13px' }}><i className="fas fa-spinner fa-spin" /> Loading...</div>;
  }
  const grouped = groupItems(items);
  if (grouped.length === 0) {
    return <div style={{ padding: '16px', textAlign: 'center', color: '#9ca3af', fontSize: '13px' }}>No items stored here</div>;
  }

  // Стили как на скриншоте 25.bmp
  const getStatusStyle = (status) => {
    if (status === 'Opened') {
      return { bg: '#fef3c7', color: '#d97706', border: '1px solid #fde68a', icon: 'fa-box-open' };
    }
    return { bg: '#d1fae5', color: '#059669', border: '1px solid #a7f3d0', icon: 'fa-lock' };
  };

  return (
    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
      <thead>
        <tr style={{ borderBottom: '2px solid #e5e7eb' }}>
          <th style={{ textAlign: 'left', padding: '6px 8px', color: '#6b7280', fontWeight: '600' }}>Reagent</th>
          <th style={{ textAlign: 'left', padding: '6px 8px', color: '#6b7280', fontWeight: '600' }}>Batch #</th>
          <th style={{ textAlign: 'right', padding: '6px 6px', color: '#6b7280', fontWeight: '600' }}>Qty</th>
          <th style={{ textAlign: 'center', padding: '6px 8px', color: '#6b7280', fontWeight: '600' }}>Containers</th>
          {showPosition && <th style={{ textAlign: 'left', padding: '6px 8px', color: '#6b7280', fontWeight: '600' }}>Position</th>}
          <th style={{ textAlign: 'center', padding: '6px 8px', color: '#6b7280', fontWeight: '600' }}>Status</th>
        </tr>
      </thead>
      <tbody>
        {grouped.map((item, i) => {
          const sStyle = getStatusStyle(item.computed_status);
          return (
            <tr key={i} style={{ borderBottom: '1px solid #f3f4f6' }}>
              <td style={{ padding: '5px 8px', color: '#1f2937', fontWeight: '500' }}>{item.reagent_name}</td>
              <td style={{ padding: '5px 8px', color: '#6b7280', fontFamily: 'monospace', fontSize: '11px' }}>{item.batch_number}</td>
              <td style={{ padding: '5px 8px', textAlign: 'right', color: '#374151' }}>
                {item.is_open
                  ? <>{fmtQty(item.computed_quantity)} {item.unit}</>
                  : <>{fmtQty(item.computed_quantity)}{item.unit} × {item.count}</>
                }
              </td>
              <td style={{ padding: '5px 8px', textAlign: 'center', color: '#6b7280' }}>
                {item.count === 1
                  ? <span>1</span>
                  : <span style={{ fontWeight: '500', color: '#374151' }}>{item.count}</span>
                }
              </td>
              {showPosition && (
                <td style={{ padding: '5px 8px', color: '#6b7280', fontSize: '11px' }}>
                  {item.position_name}{item.position_label ? ` (${item.position_label})` : ''}
                </td>
              )}
              <td style={{ padding: '5px 8px', textAlign: 'center' }}>
                <span style={{
                  padding: '3px 8px', borderRadius: '6px', fontSize: '10px', fontWeight: '600',
                  backgroundColor: sStyle.bg, color: sStyle.color, border: sStyle.border,
                  display: 'inline-flex', alignItems: 'center', gap: '4px'
                }}>
                  <i className={`fas ${sStyle.icon}`} style={{ fontSize: '9px' }} />
                  {item.computed_status}
                </span>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
};
// ==================== КОМПАКТНЫЙ СПИСОК ДЛЯ ПОЗИЦИИ ====================

const PositionItemsList = ({ items, loading }) => {
  if (loading) {
    return <div style={{ padding: '6px 0', textAlign: 'center', color: '#9ca3af', fontSize: '11px' }}><i className="fas fa-spinner fa-spin" /> Loading...</div>;
  }
  const grouped = groupItems(items);
  if (grouped.length === 0) {
    return <div style={{ padding: '6px 0', color: '#9ca3af', fontSize: '11px' }}>Empty</div>;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
      {grouped.map((item, i) => (
        <div key={i} style={{
          display: 'flex', alignItems: 'center', gap: '6px',
          padding: '3px 0', fontSize: '11px', color: '#374151',
        }}>
          <span style={{
            width: '6px', height: '6px', borderRadius: '50%', flexShrink: 0,
            backgroundColor: item.computed_status === 'Opened' ? '#f59e0b' : '#10b981',
          }} />
          <span style={{ fontWeight: '500', flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {item.reagent_name}
          </span>
          <span style={{ color: '#6b7280', whiteSpace: 'nowrap', fontSize: '10px' }}>
            {item.is_open
              ? `${fmtQty(item.computed_quantity)} ${item.unit}`
              : item.count > 1
                ? `${item.count}× ${fmtQty(item.computed_quantity)} ${item.unit}`
                : `${fmtQty(item.computed_quantity)} ${item.unit}`
            }
          </span>
        </div>
      ))}
    </div>
  );
};
// ==================== МОДАЛ ИНВЕНТАРЯ ЗОНЫ ====================

const ZoneInventoryModal = ({ zone, onClose }) => {
  const [items, setItems] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const data = await api.getZoneItems(zone.id);
        setItems(Array.isArray(data) ? data : []);
      } catch (err) {
        console.error('Failed to load zone inventory:', err);
      } finally {
        setLoading(false);
      }
    })();
  }, [zone.id]);

  const typeInfo = getZoneTypeInfo(zone.zone_type);
  const grouped = groupItems(items);

  return (
    <div style={S.formOverlay} onClick={onClose}>
      <div style={{ ...S.formModal, width: '700px', maxWidth: '95vw' }} onClick={e => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div style={{ ...S.formTitle, marginBottom: 0 }}>
            <i className={`fas ${typeInfo.icon}`} style={{ color: typeInfo.color }} />
            {zone.name} — Inventory
          </div>
          <button onClick={onClose} style={{ background: 'none', border: 'none', fontSize: '20px', cursor: 'pointer', color: '#6b7280', padding: 0, lineHeight: 1 }}>×</button>
        </div>

        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '12px' }}>
          {loading ? '...' : `${grouped.length} line${grouped.length !== 1 ? 's' : ''} (${items.length} container${items.length !== 1 ? 's' : ''} total)`}
        </div>

        <div style={{ maxHeight: '450px', overflow: 'auto' }}>
          <ItemsTable items={items} loading={loading} showPosition={true} />
        </div>

        <div style={S.formActions}>
          <button style={S.btnSecondary} onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
};

// ==================== КАРТОЧКА ПОЗИЦИИ ====================

const PositionCard = ({ position, onEdit, onDelete }) => {
  const [hovered, setHovered] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [items, setItems] = useState(null);
  const [loadingItems, setLoadingItems] = useState(false);
  const pct = position.max_capacity 
    ? Math.min(100, (position.current_count / position.max_capacity) * 100) 
    : 0;
  const fillColor = pct >= 90 ? '#ef4444' : pct >= 70 ? '#f59e0b' : '#10b981';

  const handleToggle = async () => {
    if (!expanded && items === null) {
      setLoadingItems(true);
      try {
        const data = await api.getPositionItems(position.id);
        setItems(Array.isArray(data) ? data : []);
      } catch (err) {
        console.error('Failed to load position items:', err);
        setItems([]);
      } finally {
        setLoadingItems(false);
      }
    }
    setExpanded(e => !e);
  };

  return (
    <div
      style={{
        ...S.positionCard,
        borderColor: hovered ? '#6366f1' : '#e5e7eb',
        boxShadow: hovered ? '0 2px 8px rgba(99,102,241,0.15)' : 'none',
        cursor: 'pointer',
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={handleToggle}
    >
      {/* Action buttons */}
      <div style={{ ...S.actionMenu, opacity: hovered ? 1 : 0 }}>
        <button
          style={S.actionBtn}
          onClick={(e) => { e.stopPropagation(); onEdit(position); }}
          title="Edit"
        >
          <i className="fas fa-pen" />
        </button>
        <button
          style={{ ...S.actionBtn, color: '#dc2626' }}
          onClick={(e) => {
            e.stopPropagation();
            if (window.confirm(`Delete position "${position.name}"?`)) onDelete(position.id);
          }}
          title="Delete"
        >
          <i className="fas fa-trash" />
        </button>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
        <div style={S.positionName}>{position.name}</div>
        {position.current_count > 0 && (
          <i className={`fas fa-chevron-${expanded ? 'up' : 'down'}`} style={{ fontSize: '9px', color: '#9ca3af' }} />
        )}
      </div>
      {position.position_label && (
        <div style={S.positionLabel}>
          <i className="fas fa-tag" style={{ marginRight: '3px', fontSize: '9px' }} />
          {position.position_label}
        </div>
      )}
      <div style={S.positionCount}>
        <i className="fas fa-flask" style={{ fontSize: '10px' }} />
        {position.current_count} item{position.current_count !== 1 ? 's' : ''}
        {position.max_capacity && (
          <span> / {position.max_capacity}</span>
        )}
      </div>
      {position.max_capacity && (
        <div style={S.capacityBar}>
          <div style={{ ...S.capacityFill, width: `${pct}%`, backgroundColor: fillColor }} />
        </div>
      )}

      {/* Inline items list */}
      {expanded && (
        <div style={{ marginTop: '8px', borderTop: '1px solid #e5e7eb', paddingTop: '6px' }} onClick={e => e.stopPropagation()}>
          <PositionItemsList items={items || []} loading={loadingItems} />
        </div>
      )}
    </div>
  );
};

// ==================== КАРТОЧКА ЗОНЫ ====================

const ZoneSection = ({ zone, positions, onRefresh }) => {
  const [expanded, setExpanded] = useState(false);
  const [showZoneForm, setShowZoneForm] = useState(false);
  const [showPositionForm, setShowPositionForm] = useState(false);
  const [showInventory, setShowInventory] = useState(false);
  const [editPosition, setEditPosition] = useState(null);
  const [hovered, setHovered] = useState(false);

  const typeInfo = getZoneTypeInfo(zone.zone_type);
  const totalItems = positions.reduce((sum, p) => sum + p.current_count, 0);

  const handleDeleteZone = async () => {
    if (!window.confirm(`Delete zone "${zone.name}" and all its positions?`)) return;
    try {
      await api.deleteStorageZone(zone.id);
      onRefresh();
    } catch (err) {
      alert(err.message || 'Cannot delete zone');
    }
  };

  const handleDeletePosition = async (posId) => {
    try {
      await api.deleteStoragePosition(posId);
      onRefresh();
    } catch (err) {
      alert(err.message || 'Cannot delete position');
    }
  };

  return (
    <div style={S.zoneCard}>
      <div
        style={{ ...S.zoneHeader, backgroundColor: hovered ? '#f3f4f6' : 'transparent' }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onClick={() => setExpanded(e => !e)}
      >
        <div style={{ ...S.zoneIcon, backgroundColor: typeInfo.color }}>
          <i className={`fas ${typeInfo.icon}`} />
        </div>
        <div style={{ flex: 1 }}>
          <div style={S.zoneName}>
            {zone.name}
            <span style={S.zoneType}> — {typeInfo.label}</span>
          </div>
          <div style={S.zoneMeta}>
            {positions.length} position{positions.length !== 1 ? 's' : ''} · {totalItems} item{totalItems !== 1 ? 's' : ''}
            {zone.storage_condition && (
              <> · {STORAGE_CONDITIONS.find(c => c.value === zone.storage_condition)?.label || zone.storage_condition}</>
            )}
            {zone.is_locked === 1 && <> · <i className="fas fa-lock" style={{ fontSize: '10px' }} /> Locked</>}
          </div>
        </div>
        
        {/* Zone actions */}
        <div style={{ display: 'flex', gap: '4px', opacity: hovered ? 1 : 0, transition: 'opacity 0.15s' }}
             onClick={e => e.stopPropagation()}>
          <button style={{ ...S.btnSmall, color: '#6366f1' }} onClick={() => setShowInventory(true)} title="View inventory">
            <i className="fas fa-eye" style={{ fontSize: '10px' }} />
          </button>
          <button style={S.btnSmall} onClick={() => setShowZoneForm(true)} title="Edit zone">
            <i className="fas fa-pen" style={{ fontSize: '10px' }} />
          </button>
          <button style={{ ...S.btnSmall, color: '#dc2626' }} onClick={handleDeleteZone} title="Delete zone">
            <i className="fas fa-trash" style={{ fontSize: '10px' }} />
          </button>
        </div>

        <i
          className="fas fa-chevron-right"
          style={{ ...S.chevron, transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
        />
      </div>

      {expanded && (
        <div style={S.positionsGrid}>
          {positions.map(pos => (
            <PositionCard
              key={pos.id}
              position={pos}
              onEdit={(p) => { setEditPosition(p); setShowPositionForm(true); }}
              onDelete={handleDeletePosition}
            />
          ))}
          <div
            style={S.addPositionBtn}
            onClick={() => { setEditPosition(null); setShowPositionForm(true); }}
            onMouseEnter={e => { e.currentTarget.style.borderColor = '#10b981'; e.currentTarget.style.color = '#10b981'; }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = '#d1d5db'; e.currentTarget.style.color = '#9ca3af'; }}
          >
            <i className="fas fa-plus" style={{ fontSize: '11px' }} />
            Add Position
          </div>
        </div>
      )}

      {/* Zone edit form */}
      {showZoneForm && (
        <ZoneForm
          roomId={zone.room_id}
          zone={zone}
          onSave={() => { setShowZoneForm(false); onRefresh(); }}
          onCancel={() => setShowZoneForm(false)}
        />
      )}

      {/* Position form */}
      {showPositionForm && (
        <PositionForm
          zoneId={zone.id}
          position={editPosition}
          onSave={() => { setShowPositionForm(false); onRefresh(); }}
          onCancel={() => { setShowPositionForm(false); setEditPosition(null); }}
        />
      )}

      {/* Zone inventory modal */}
      {showInventory && (
        <ZoneInventoryModal
          zone={zone}
          onClose={() => setShowInventory(false)}
        />
      )}
    </div>
  );
};

// ==================== КАРТОЧКА КОМНАТЫ ====================

const RoomSection = ({ room, onRefresh }) => {
  const [expanded, setExpanded] = useState(false);
  const [showZoneForm, setShowZoneForm] = useState(false);
  const [showRoomForm, setShowRoomForm] = useState(false);
  const [hovered, setHovered] = useState(false);

  const zoneCount = room.zones?.length || 0;
  const posCount = room.zones?.reduce((s, z) => s + (z.positions?.length || 0), 0) || 0;

  const handleDeleteRoom = async () => {
    if (!window.confirm(`Delete room "${room.name}" and all its storage zones?`)) return;
    try {
      await api.deleteRoom(room.id);
      onRefresh();
    } catch (err) {
      alert(err.message || 'Cannot delete room');
    }
  };

  return (
    <div style={{ ...S.roomCard, boxShadow: hovered ? '0 4px 12px rgba(0,0,0,0.06)' : 'none' }}>
      <div
        style={S.roomHeader}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onClick={() => setExpanded(e => !e)}
      >
        <div style={{ ...S.roomColorBar, backgroundColor: room.color || '#667eea' }} />
        <div style={{ flex: 1 }}>
          <div style={S.roomName}>{room.name}</div>
          {room.description && (
            <div style={{ fontSize: '12px', color: '#9ca3af', marginTop: '2px' }}>{room.description}</div>
          )}
        </div>
        <div style={S.roomMeta}>
          {room.capacity && (
            <span>
              <i className="fas fa-users" style={{ marginRight: '4px' }} />
              {room.capacity}
            </span>
          )}
          <span>
            <i className="fas fa-th-large" style={{ marginRight: '4px' }} />
            {zoneCount} zone{zoneCount !== 1 ? 's' : ''}
          </span>
          <span>
            <i className="fas fa-bookmark" style={{ marginRight: '4px' }} />
            {posCount} position{posCount !== 1 ? 's' : ''}
          </span>
          <span>
            <i className="fas fa-flask" style={{ marginRight: '4px' }} />
            {room.total_items} item{room.total_items !== 1 ? 's' : ''}
          </span>
        </div>

        {/* Room actions */}
        <div style={{ display: 'flex', gap: '4px', opacity: hovered ? 1 : 0, transition: 'opacity 0.15s' }}
             onClick={e => e.stopPropagation()}>
          <button style={S.btnSmall} onClick={() => setShowRoomForm(true)} title="Edit room">
            <i className="fas fa-pen" style={{ fontSize: '10px' }} />
          </button>
          <button style={{ ...S.btnSmall, color: '#dc2626' }} onClick={handleDeleteRoom} title="Delete room">
            <i className="fas fa-trash" style={{ fontSize: '10px' }} />
          </button>
        </div>

        <i
          className="fas fa-chevron-right"
          style={{ ...S.chevron, transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
        />
      </div>

      {expanded && (
        <div style={S.zonesContainer}>
          {room.zones?.length === 0 && (
            <div style={{ textAlign: 'center', padding: '20px', color: '#9ca3af', fontSize: '13px' }}>
              No storage zones yet. Add a cabinet, fridge, or shelf to get started.
            </div>
          )}

          {room.zones?.map(zoneData => (
            <ZoneSection
              key={zoneData.id}
              zone={zoneData}
              positions={zoneData.positions}
              onRefresh={onRefresh}
            />
          ))}

          <button
            style={S.addZoneBtn}
            onClick={(e) => { e.stopPropagation(); setShowZoneForm(true); }}
            onMouseEnter={e => { e.currentTarget.style.borderColor = '#6366f1'; e.currentTarget.style.color = '#6366f1'; }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = '#d1d5db'; e.currentTarget.style.color = '#6b7280'; }}
          >
            <i className="fas fa-plus" /> Add Storage Zone
          </button>
        </div>
      )}

      {showZoneForm && (
        <ZoneForm
          roomId={room.id}
          zone={null}
          onSave={() => { setShowZoneForm(false); onRefresh(); }}
          onCancel={() => setShowZoneForm(false)}
        />
      )}

      {showRoomForm && (
        <RoomForm
          room={room}
          onSave={() => { setShowRoomForm(false); onRefresh(); }}
          onCancel={() => setShowRoomForm(false)}
        />
      )}
    </div>
  );
};

// ==================== ГЛАВНЫЙ КОМПОНЕНТ ====================

const LocationManager = () => {
  const [hierarchy, setHierarchy] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [showRoomForm, setShowRoomForm] = useState(false);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const data = await api.getStorageHierarchy();
      setHierarchy(data || []);
    } catch (err) {
      console.error('Failed to load storage hierarchy:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  // Фильтрация по поисковому запросу
  const filtered = useMemo(() => {
    if (!search.trim()) return hierarchy;
    const q = search.toLowerCase();
    return hierarchy
      .map(room => ({
        ...room,
        zones: room.zones
          .map(zoneData => ({
            ...zoneData,
            positions: zoneData.positions.filter(p =>
              p.name.toLowerCase().includes(q) ||
              (p.position_label && p.position_label.toLowerCase().includes(q))
            ),
          }))
          .filter(zoneData =>
            zoneData.name.toLowerCase().includes(q) ||
            zoneData.positions.length > 0
          ),
      }))
      .filter(room =>
        room.name.toLowerCase().includes(q) ||
        room.zones.length > 0
      );
  }, [hierarchy, search]);

  // Статистика
  const stats = useMemo(() => {
    const rooms = hierarchy.length;
    const zones = hierarchy.reduce((s, r) => s + (r.zones?.length || 0), 0);
    const positions = hierarchy.reduce((s, r) =>
      s + (r.zones?.reduce((s2, z) => s2 + (z.positions?.length || 0), 0) || 0), 0);
    const items = hierarchy.reduce((s, r) => s + (r.total_items || 0), 0);
    return { rooms, zones, positions, items };
  }, [hierarchy]);

  if (loading) {
    return <div style={S.loading}><i className="fas fa-spinner fa-spin" /> Loading storage locations...</div>;
  }

  return (
    <div style={S.container}>
      {/* Header */}
      <div style={S.header}>
        <div>
          <div style={S.title}>
            <i className="fas fa-warehouse" style={{ color: '#6366f1' }} />
            Storage Locations
          </div>
          <div style={S.subtitle}>
            Manage rooms, cabinets, shelves, and track where every reagent is stored
          </div>
        </div>
        <div style={S.toolbar}>
          <input
            style={S.searchBox}
            placeholder="Search locations..."
            value={search}
            onChange={e => setSearch(e.target.value)}
            onFocus={e => { e.target.style.borderColor = '#6366f1'; e.target.style.boxShadow = '0 0 0 3px rgba(99,102,241,0.1)'; }}
            onBlur={e => { e.target.style.borderColor = '#e5e7eb'; e.target.style.boxShadow = 'none'; }}
          />
          <button
            style={S.addBtn}
            onClick={() => setShowRoomForm(true)}
            onMouseEnter={e => { e.currentTarget.style.backgroundColor = '#4f46e5'; }}
            onMouseLeave={e => { e.currentTarget.style.backgroundColor = '#6366f1'; }}
          >
            <i className="fas fa-plus" /> Add Room
          </button>
        </div>
      </div>

      {/* Stats */}
      <div style={S.statsBar}>
        <div style={S.statItem}>
          <i className="fas fa-door-open" style={{ color: '#6366f1' }} />
          <span style={S.statValue}>{stats.rooms}</span> Room{stats.rooms !== 1 ? 's' : ''}
        </div>
        <div style={S.statItem}>
          <i className="fas fa-box" style={{ color: '#f59e0b' }} />
          <span style={S.statValue}>{stats.zones}</span> Zone{stats.zones !== 1 ? 's' : ''}
        </div>
        <div style={S.statItem}>
          <i className="fas fa-bookmark" style={{ color: '#10b981' }} />
          <span style={S.statValue}>{stats.positions}</span> Position{stats.positions !== 1 ? 's' : ''}
        </div>
        <div style={S.statItem}>
          <i className="fas fa-flask" style={{ color: '#6b7280' }} />
          <span style={S.statValue}>{stats.items}</span> Stored Item{stats.items !== 1 ? 's' : ''}
        </div>
      </div>

      {/* Room list */}
      {filtered.length === 0 ? (
        <div style={S.emptyState}>
          <div style={S.emptyIcon}><i className="fas fa-warehouse" /></div>
          <div style={S.emptyText}>
            {search ? 'No locations match your search' : 'No rooms configured yet'}
          </div>
          <div style={S.emptyHint}>
            {search ? 'Try a different search term' : 'Click "Add Room" above to create your first lab room'}
          </div>
          {!search && (
            <button
              style={{ ...S.addBtn, marginTop: '16px' }}
              onClick={() => setShowRoomForm(true)}
              onMouseEnter={e => { e.currentTarget.style.backgroundColor = '#4f46e5'; }}
              onMouseLeave={e => { e.currentTarget.style.backgroundColor = '#6366f1'; }}
            >
              <i className="fas fa-plus" /> Add Room
            </button>
          )}
        </div>
      ) : (
        filtered.map(room => (
          <RoomSection
            key={room.id}
            room={room}
            onRefresh={loadData}
          />
        ))
      )}

      {/* Room Form Modal */}
      {showRoomForm && (
        <RoomForm
          room={null}
          onSave={() => { setShowRoomForm(false); loadData(); }}
          onCancel={() => setShowRoomForm(false)}
        />
      )}
    </div>
  );
};

export default LocationManager;