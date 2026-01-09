// src/components/experiments/RoomManager.js
import React from 'react';
import { useRoomManager } from './hooks/useRoomManager';
import styles from './RoomManager.module.css';

// === Подкомпоненты ===

const AddRoomForm = ({ newRoom, loading, onFieldChange, onAdd }) => {
  const handleAdd = async () => {
    const result = await onAdd();
    if (!result.success) {
      alert(result.message || 'Failed to add room');
    }
  };

  return (
    <div className={styles.addForm}>
      <h3 className={styles.formTitle}>Add New Room</h3>
      <div className={styles.formGrid}>
        <div className={styles.formField}>
          <label className={styles.label}>Name *</label>
          <input
            type="text"
            value={newRoom.name}
            onChange={(e) => onFieldChange('name', e.target.value)}
            placeholder="e.g., Lab 104"
            className={styles.input}
          />
        </div>
        <div className={styles.formField}>
          <label className={styles.label}>Color</label>
          <input
            type="color"
            value={newRoom.color}
            onChange={(e) => onFieldChange('color', e.target.value)}
            className={styles.colorInput}
          />
        </div>
        <div className={styles.formField}>
          <label className={styles.label}>Capacity</label>
          <input
            type="number"
            value={newRoom.capacity}
            onChange={(e) => onFieldChange('capacity', e.target.value)}
            placeholder="20"
            className={styles.input}
          />
        </div>
        <div className={styles.formField}>
          <label className={styles.label}>Description</label>
          <input
            type="text"
            value={newRoom.description}
            onChange={(e) => onFieldChange('description', e.target.value)}
            placeholder="Optional"
            className={styles.input}
          />
        </div>
        <button
          onClick={handleAdd}
          disabled={loading}
          className={styles.addButton}
        >
          <i className="fas fa-plus"></i> Add
        </button>
      </div>
    </div>
  );
};

const RoomItemEditing = ({ 
  room, 
  editingRoom, 
  onFieldChange, 
  onSave, 
  onCancel 
}) => {
  const handleSave = async () => {
    const result = await onSave(room.id);
    if (!result.success) {
      alert(result.message || 'Failed to update room');
    }
  };

  return (
    <>
      <input
        type="text"
        value={editingRoom.name}
        onChange={(e) => onFieldChange('name', e.target.value)}
        className={styles.editInput}
      />
      <input
        type="color"
        value={editingRoom.color || '#667eea'}
        onChange={(e) => onFieldChange('color', e.target.value)}
        className={styles.editColorInput}
      />
      <button onClick={handleSave} className={styles.saveButton}>
        Save
      </button>
      <button onClick={onCancel} className={styles.cancelButton}>
        Cancel
      </button>
    </>
  );
};

const RoomItemDisplay = ({ room, onEdit, onDelete }) => {
  const handleDelete = async () => {
    if (!window.confirm('Are you sure you want to delete this room?')) {
      return;
    }
    const result = await onDelete(room.id);
    if (!result.success) {
      alert(result.message || 'Failed to delete room');
    }
  };

  return (
    <>
      <div 
        className={styles.colorDot} 
        style={{ backgroundColor: room.color || '#667eea' }}
      />
      <div className={styles.roomInfo}>
        <div className={styles.roomName}>
          {room.name}
          {room.capacity && (
            <span className={styles.roomCapacity}>
              ({room.capacity} seats)
            </span>
          )}
        </div>
        {room.description && (
          <div className={styles.roomDescription}>{room.description}</div>
        )}
      </div>
      <button 
        onClick={() => onEdit(room)} 
        className={styles.editButton}
        title="Edit room"
      >
        <i className="fas fa-edit"></i>
      </button>
      <button 
        onClick={handleDelete} 
        className={styles.deleteButton}
        title="Delete room"
      >
        <i className="fas fa-trash"></i>
      </button>
    </>
  );
};

const RoomItem = ({ 
  room, 
  isEditing, 
  editingRoom, 
  onEditFieldChange, 
  onSave, 
  onCancel, 
  onStartEdit, 
  onDelete 
}) => (
  <div 
    className={styles.roomItem}
    style={{ borderLeftColor: room.color || '#667eea' }}
  >
    {isEditing ? (
      <RoomItemEditing
        room={room}
        editingRoom={editingRoom}
        onFieldChange={onEditFieldChange}
        onSave={onSave}
        onCancel={onCancel}
      />
    ) : (
      <RoomItemDisplay
        room={room}
        onEdit={onStartEdit}
        onDelete={onDelete}
      />
    )}
  </div>
);

const RoomList = ({ 
  rooms, 
  editingRoom, 
  onEditFieldChange, 
  onSave, 
  onCancel, 
  onStartEdit, 
  onDelete 
}) => (
  <div className={styles.roomList}>
    {rooms.length === 0 ? (
      <div className={styles.emptyMessage}>
        No rooms added yet. Add your first room above.
      </div>
    ) : (
      rooms.map(room => (
        <RoomItem
          key={room.id}
          room={room}
          isEditing={editingRoom?.id === room.id}
          editingRoom={editingRoom}
          onEditFieldChange={onEditFieldChange}
          onSave={onSave}
          onCancel={onCancel}
          onStartEdit={onStartEdit}
          onDelete={onDelete}
        />
      ))
    )}
  </div>
);

// === Основной компонент ===

const RoomManager = ({ rooms: initialRooms, onClose, onUpdate }) => {
  const {
    rooms,
    loading,
    newRoom,
    editingRoom,
    addRoom,
    updateRoom,
    deleteRoom,
    startEditing,
    cancelEditing,
    updateNewRoomField,
    updateEditingRoomField,
  } = useRoomManager(initialRooms, onUpdate);

  return (
    <div className={styles.overlay}>
      <div className={styles.modal}>
        {/* Header */}
        <div className={styles.header}>
          <h2 className={styles.title}>
            <i className="fas fa-door-open" style={{ marginRight: '10px', color: '#8b5cf6' }}></i>
            Manage Rooms
          </h2>
          <button onClick={onClose} className={styles.closeButton}>
            ×
          </button>
        </div>

        {/* Add Form */}
        <AddRoomForm
          newRoom={newRoom}
          loading={loading}
          onFieldChange={updateNewRoomField}
          onAdd={addRoom}
        />

        {/* Room List */}
        <RoomList
          rooms={rooms}
          editingRoom={editingRoom}
          onEditFieldChange={updateEditingRoomField}
          onSave={updateRoom}
          onCancel={cancelEditing}
          onStartEdit={startEditing}
          onDelete={deleteRoom}
        />

        {/* Footer */}
        <div className={styles.footer}>
          <button onClick={onClose} className={styles.doneButton}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
};

export default RoomManager;
