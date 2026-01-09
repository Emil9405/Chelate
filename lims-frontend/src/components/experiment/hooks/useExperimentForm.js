// src/components/experiments/hooks/useExperimentForm.js
import { useState, useCallback } from 'react';
import { api } from '../../../services/api';

/**
 * Начальное состояние формы
 */
const INITIAL_FORM_DATA = {
  title: '',
  description: '',
  experiment_date: '',
  experiment_type: 'research',
  instructor: '',
  student_group: '',
  location: '',
  room_id: '',
  start_time: '',
  end_time: '',
};

/**
 * Утилиты для работы с датами
 */
const dateUtils = {
  // UTC ISO строка → локальное время HH:MM
  utcToLocalTime: (isoString) => {
    if (!isoString) return '';
    try {
      const date = new Date(isoString);
      const hours = date.getHours().toString().padStart(2, '0');
      const minutes = date.getMinutes().toString().padStart(2, '0');
      return `${hours}:${minutes}`;
    } catch {
      return '';
    }
  },

  // UTC ISO строка → локальная дата YYYY-MM-DD
  utcToLocalDate: (isoString) => {
    if (!isoString) return '';
    try {
      const date = new Date(isoString);
      const year = date.getFullYear();
      const month = (date.getMonth() + 1).toString().padStart(2, '0');
      const day = date.getDate().toString().padStart(2, '0');
      return `${year}-${month}-${day}`;
    } catch {
      return isoString.split('T')[0];
    }
  },

  // Локальная дата + время → UTC ISO строка
  toUTCString: (dateStr, timeStr) => {
    const [y, m, d] = dateStr.split('-').map(Number);
    if (timeStr) {
      const [hours, minutes] = timeStr.split(':').map(Number);
      const localDate = new Date(y, m - 1, d, hours, minutes, 0);
      return localDate.toISOString();
    } else {
      const localDate = new Date(y, m - 1, d, 12, 0, 0);
      return localDate.toISOString();
    }
  },
};

/**
 * Валидация времени
 */
const validateTime = (formData) => {
  if (formData.experiment_type !== 'educational') {
    return { valid: true };
  }

  if (!formData.start_time || !formData.end_time) {
    return { valid: false, message: 'Educational experiments require start and end time' };
  }

  const timeRegex = /^([01]?[0-9]|2[0-3]):([0-5][0-9])$/;
  if (!timeRegex.test(formData.start_time) || !timeRegex.test(formData.end_time)) {
    return { valid: false, message: 'Invalid time format. Use HH:MM (00:00 - 23:59)' };
  }

  const [startH, startM] = formData.start_time.split(':').map(Number);
  const [endH, endM] = formData.end_time.split(':').map(Number);
  const startMinutes = startH * 60 + startM;
  const endMinutes = endH * 60 + endM;

  if (endMinutes <= startMinutes) {
    return { valid: false, message: 'End time must be after start time' };
  }

  const durationMinutes = endMinutes - startMinutes;
  if (durationMinutes < 15 || durationMinutes > 480) {
    return { valid: false, message: 'Experiment duration must be between 15 minutes and 8 hours' };
  }

  return { valid: true };
};

/**
 * Форматирование времени при вводе
 */
const formatTimeInput = (value) => {
  let cleaned = value.replace(/[^\d:]/g, '');

  if (cleaned.length > 5) {
    cleaned = cleaned.substring(0, 5);
  }

  // Автодобавление двоеточия после двух цифр
  if (cleaned.length === 2 && !cleaned.includes(':')) {
    const hours = parseInt(cleaned);
    if (hours > 23) {
      cleaned = '23';
    }
    cleaned = cleaned + ':';
  }

  // Валидация часов
  if (cleaned.length >= 2) {
    const hoursPart = cleaned.split(':')[0];
    if (hoursPart && parseInt(hoursPart) > 23) {
      cleaned = '23' + cleaned.substring(2);
    }
  }

  // Валидация минут
  if (cleaned.includes(':')) {
    const parts = cleaned.split(':');
    if (parts[1] && parts[1].length === 2) {
      const minutes = parseInt(parts[1]);
      if (minutes > 59) {
        cleaned = parts[0] + ':59';
      }
    }
  }

  return cleaned;
};

/**
 * Хук для управления формой эксперимента
 */
export const useExperimentForm = ({ rooms, onSuccess, onCancel }) => {
  const [formData, setFormData] = useState(INITIAL_FORM_DATA);
  const [editingId, setEditingId] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const isEditing = !!editingId;

  // === Обработчики изменений ===
  const handleInputChange = useCallback((e) => {
    const { name, value } = e.target;
    setFormData(prev => ({ ...prev, [name]: value }));
    setError(null);
  }, []);

  const handleTimeChange = useCallback((e) => {
    const { name, value } = e.target;
    const formatted = formatTimeInput(value);
    setFormData(prev => ({ ...prev, [name]: formatted }));
    setError(null);
  }, []);

  const handleRoomChange = useCallback((roomId) => {
    const room = rooms.find(r => r.id === roomId);
    setFormData(prev => ({
      ...prev,
      room_id: roomId,
      location: room ? room.name : prev.location,
    }));
    setError(null);
  }, [rooms]);

  // === Загрузка данных для редактирования ===
  const loadExperiment = useCallback((experiment) => {
    const room = rooms.find(r => r.name === experiment.location);

    setFormData({
      title: experiment.title,
      description: experiment.description || '',
      experiment_date: dateUtils.utcToLocalDate(experiment.experiment_date),
      experiment_type: experiment.experiment_type || 'research',
      instructor: experiment.instructor || '',
      student_group: experiment.student_group || '',
      location: experiment.location || '',
      room_id: room?.id || experiment.room_id || '',
      start_time: dateUtils.utcToLocalTime(experiment.start_date),
      end_time: dateUtils.utcToLocalTime(experiment.end_date),
    });
    setEditingId(experiment.id);
    setError(null);
  }, [rooms]);

  // === Очистка формы (без закрытия) ===
  const clearForm = useCallback(() => {
    setFormData(INITIAL_FORM_DATA);
    setEditingId(null);
    setError(null);
  }, []);

  // === Сброс формы с закрытием ===
  const resetForm = useCallback(() => {
    clearForm();
    onCancel?.();
  }, [clearForm, onCancel]);

  // === Отправка формы ===
  const handleSubmit = useCallback(async (e) => {
    e?.preventDefault();
    setError(null);

    // Базовая валидация
    if (!formData.title || !formData.experiment_date) {
      setError('Please fill in required fields (Title and Date)');
      return { success: false, message: 'Please fill in required fields' };
    }

    // Валидация времени для educational
    const timeValidation = validateTime(formData);
    if (!timeValidation.valid) {
      setError(timeValidation.message);
      return { success: false, message: timeValidation.message };
    }

    setLoading(true);

    try {
      // Находим имя комнаты по room_id
      const selectedRoom = rooms.find(r => r.id === formData.room_id);
      const locationValue = selectedRoom ? selectedRoom.name : formData.location;

      const dataToSubmit = {
        title: formData.title,
        description: formData.description || '',
        experiment_date: dateUtils.toUTCString(formData.experiment_date, formData.start_time || '12:00'),
        experiment_type: formData.experiment_type || 'research',
        instructor: formData.instructor || '',
        student_group: formData.student_group || '',
        location: locationValue,
        room_id: formData.room_id || null,
      };

      if (formData.start_time) {
        dataToSubmit.start_date = dateUtils.toUTCString(formData.experiment_date, formData.start_time);
      }
      if (formData.end_time) {
        dataToSubmit.end_date = dateUtils.toUTCString(formData.experiment_date, formData.end_time);
      }

      if (editingId) {
        await api.updateExperiment(editingId, dataToSubmit);
      } else {
        await api.createExperiment(dataToSubmit);
      }

      const message = editingId ? 'Experiment updated successfully' : 'Experiment created successfully';
      clearForm();
      onSuccess?.();

      return { success: true, message };
    } catch (err) {
      console.error('Error saving experiment:', err);
      setError(err.message);
      return { success: false, message: err.message };
    } finally {
      setLoading(false);
    }
  }, [formData, editingId, rooms, clearForm, onSuccess]);

  // === Обновить конкретное поле ===
  const setField = useCallback((field, value) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    setError(null);
  }, []);

  return {
    // Состояние
    formData,
    editingId,
    isEditing,
    loading,
    error,

    // Обработчики
    handleInputChange,
    handleTimeChange,
    handleRoomChange,
    handleSubmit,
    setField,

    // Действия
    loadExperiment,
    clearForm,    // Очистка без закрытия
    resetForm,    // Очистка + закрытие
  };
};

export default useExperimentForm;
