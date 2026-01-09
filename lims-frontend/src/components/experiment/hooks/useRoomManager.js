// src/components/experiments/hooks/useRoomManager.js
import { useState, useCallback } from 'react';
import { api } from '../../../services/api';

/**
 * Хук для управления комнатами лаборатории
 */
export const useRoomManager = (initialRooms, onUpdate) => {
  const [rooms, setRooms] = useState(initialRooms);
  const [loading, setLoading] = useState(false);
  const [editingRoom, setEditingRoom] = useState(null);
  
  // Форма новой комнаты
  const [newRoom, setNewRoom] = useState({
    name: '',
    color: '#667eea',
    description: '',
    capacity: ''
  });

  // === Создание комнаты ===
  const addRoom = useCallback(async () => {
    if (!newRoom.name.trim()) {
      return { success: false, message: 'Please enter room name' };
    }

    setLoading(true);
    try {
      const result = await api.createRoom({
        name: newRoom.name.trim(),
        color: newRoom.color,
        description: newRoom.description.trim() || null,
        capacity: newRoom.capacity ? parseInt(newRoom.capacity) : null,
      });
      
      const createdRoom = result.data || result;
      setRooms(prev => [...prev, createdRoom]);
      setNewRoom({ name: '', color: '#667eea', description: '', capacity: '' });
      onUpdate?.();
      
      return { success: true, room: createdRoom };
    } catch (error) {
      console.error('Error creating room:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [newRoom, onUpdate]);

  // === Обновление комнаты ===
  const updateRoom = useCallback(async (roomId) => {
    if (!editingRoom || !editingRoom.name.trim()) {
      return { success: false, message: 'Please enter room name' };
    }

    setLoading(true);
    try {
      await api.updateRoom(roomId, {
        name: editingRoom.name.trim(),
        color: editingRoom.color,
        description: editingRoom.description?.trim() || null,
        capacity: editingRoom.capacity ? parseInt(editingRoom.capacity) : null,
      });
      
      setRooms(prev => prev.map(r => 
        r.id === roomId ? { ...r, ...editingRoom } : r
      ));
      setEditingRoom(null);
      onUpdate?.();
      
      return { success: true };
    } catch (error) {
      console.error('Error updating room:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [editingRoom, onUpdate]);

  // === Удаление комнаты ===
  const deleteRoom = useCallback(async (roomId) => {
    setLoading(true);
    try {
      await api.deleteRoom(roomId);
      setRooms(prev => prev.filter(r => r.id !== roomId));
      onUpdate?.();
      
      return { success: true };
    } catch (error) {
      console.error('Error deleting room:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [onUpdate]);

  // === Начать редактирование ===
  const startEditing = useCallback((room) => {
    setEditingRoom({ ...room });
  }, []);

  // === Отменить редактирование ===
  const cancelEditing = useCallback(() => {
    setEditingRoom(null);
  }, []);

  // === Обновить поле новой комнаты ===
  const updateNewRoomField = useCallback((field, value) => {
    setNewRoom(prev => ({ ...prev, [field]: value }));
  }, []);

  // === Обновить поле редактируемой комнаты ===
  const updateEditingRoomField = useCallback((field, value) => {
    setEditingRoom(prev => prev ? { ...prev, [field]: value } : null);
  }, []);

  return {
    // Состояние
    rooms,
    loading,
    newRoom,
    editingRoom,
    
    // Действия
    addRoom,
    updateRoom,
    deleteRoom,
    
    // Редактирование
    startEditing,
    cancelEditing,
    updateNewRoomField,
    updateEditingRoomField,
  };
};

export default useRoomManager;
