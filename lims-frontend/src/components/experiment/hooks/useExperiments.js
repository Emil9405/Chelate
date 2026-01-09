// src/components/experiments/hooks/useExperiments.js
import { useState, useEffect, useCallback, useMemo } from 'react';
import { api } from '../../../services/api';

/**
 * Главный хук для управления экспериментами
 * Инкапсулирует загрузку данных, фильтрацию, CRUD операции и UI состояния
 */
export const useExperiments = (user) => {
  // === Данные ===
  const [experiments, setExperiments] = useState([]);
  const [batches, setBatches] = useState([]);
  const [equipment, setEquipment] = useState([]);
  const [rooms, setRooms] = useState([]);
  const [loading, setLoading] = useState(true);

  // === Фильтры ===
  const [filters, setFilters] = useState({
    search: '',
    status: '',
    location: '',
    room: '',
    calendarStatuses: [],
  });

  // === UI состояния ===
  const [ui, setUi] = useState({
    viewMode: 'list',           // 'list' | 'calendar'
    currentDate: new Date(),
    showForm: false,
    showDetails: false,
    showRoomManager: false,
  });

  // === Выбранные элементы ===
  const [selectedExperiment, setSelectedExperiment] = useState(null);
  const [editingExperiment, setEditingExperiment] = useState(null);

  // === Загрузка данных ===
  const loadData = useCallback(async () => {
    try {
      setLoading(true);

      // Автоматически обновляем статусы экспериментов (опционально)
      try {
        await api.autoUpdateExperimentStatuses();
      } catch (e) {
        // Игнорируем ошибку если endpoint не существует
        console.log('Auto-update statuses skipped:', e.message);
      }

      // Формируем параметры запроса
      const params = {};
      if (filters.search) params.search = filters.search;
      if (filters.status) params.status = filters.status;
      if (filters.location) params.location = filters.location;

      // Параллельная загрузка всех данных
      const [experimentsData, batchesData, equipmentData, roomsData] = await Promise.all([
        api.getExperiments(params),
        api.getAllBatches(),
        api.getEquipment(),
        api.getRooms().catch(() => []),
      ]);

      setExperiments(experimentsData.data || experimentsData);
      setBatches(batchesData.data || batchesData);
      setEquipment(equipmentData.data || equipmentData);
      setRooms(roomsData.data || roomsData || []);
    } catch (error) {
      console.error('Error loading data:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
    return { success: true };
  }, [filters.search, filters.status, filters.location]);

  // Загрузка при изменении фильтров
  useEffect(() => {
    loadData();
  }, [loadData]);

  // === Фильтрация экспериментов ===
  const filteredExperiments = useMemo(() => {
    let result = experiments;

    // Поиск
    if (filters.search) {
      const searchLower = filters.search.toLowerCase();
      result = result.filter(exp =>
        exp.title.toLowerCase().includes(searchLower) ||
        (exp.description && exp.description.toLowerCase().includes(searchLower)) ||
        (exp.instructor && exp.instructor.toLowerCase().includes(searchLower)) ||
        (exp.student_group && exp.student_group.toLowerCase().includes(searchLower))
      );
    }

    // Фильтр по статусу
    if (filters.status) {
      result = result.filter(exp => exp.status === filters.status);
    }

    // Фильтр по статусам календаря (множественный выбор)
    if (filters.calendarStatuses.length > 0) {
      result = result.filter(exp => filters.calendarStatuses.includes(exp.status));
    }

    // Фильтр по комнате
    if (filters.room) {
      result = result.filter(exp => exp.location === filters.room);
    }

    return result;
  }, [experiments, filters]);

  // === Действия с экспериментами ===
  const deleteExperiment = useCallback(async (id) => {
    try {
      await api.deleteExperiment(id);
      await loadData();
      return { success: true, message: 'Experiment deleted successfully' };
    } catch (error) {
      console.error('Error deleting experiment:', error);
      return { success: false, message: error.message };
    }
  }, [loadData]);

  const updateStatus = useCallback(async (id, newStatus) => {
    try {
      await api.updateExperiment(id, { status: newStatus });
      setExperiments(prev => prev.map(exp =>
        exp.id === id ? { ...exp, status: newStatus } : exp
      ));
      return { success: true };
    } catch (error) {
      console.error('Error updating status:', error);
      return { success: false, message: error.message };
    }
  }, []);

  const viewDetails = useCallback(async (id) => {
    try {
      const details = await api.getExperimentDetails(id);
      setSelectedExperiment(details);
      setUi(prev => ({ ...prev, showDetails: true }));
      return { success: true, data: details };
    } catch (error) {
      console.error('Error loading experiment details:', error);
      return { success: false, message: error.message };
    }
  }, []);

  // Перезагрузка деталей текущего эксперимента (для обновления после изменений)
  const refreshDetails = useCallback(async () => {
    if (selectedExperiment?.experiment?.id || selectedExperiment?.id) {
      const id = selectedExperiment?.experiment?.id || selectedExperiment?.id;
      try {
        const details = await api.getExperimentDetails(id);
        setSelectedExperiment(details);
        // Также обновляем список экспериментов
        loadData();
      } catch (error) {
        console.error('Error refreshing experiment details:', error);
      }
    }
  }, [selectedExperiment, loadData]);

  const closeDetails = useCallback(() => {
    setSelectedExperiment(null);
    setUi(prev => ({ ...prev, showDetails: false }));
  }, []);

  // === Управление формой ===
  const openCreateForm = useCallback(() => {
    setEditingExperiment(null);
    setUi(prev => ({ ...prev, showForm: true, viewMode: 'list' }));
  }, []);

  const openEditForm = useCallback((experiment) => {
    setEditingExperiment(experiment);
    setUi(prev => ({ ...prev, showForm: true, viewMode: 'list' }));
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }, []);

  const closeForm = useCallback(() => {
    setEditingExperiment(null);
    setUi(prev => ({ ...prev, showForm: false }));
  }, []);

  const onFormSuccess = useCallback(() => {
    closeForm();
    loadData();
  }, [closeForm, loadData]);

  // === Управление комнатами ===
  const openRoomManager = useCallback(() => {
    setUi(prev => ({ ...prev, showRoomManager: true }));
  }, []);

  const closeRoomManager = useCallback(() => {
    setUi(prev => ({ ...prev, showRoomManager: false }));
  }, []);

  // === Утилиты ===
  const getRoomColor = useCallback((location) => {
    const room = rooms.find(r => r.name === location || r.id === location);
    return room?.color || '#667eea';
  }, [rooms]);

  // === Управление фильтрами ===
  const setFilter = useCallback((key, value) => {
    setFilters(prev => ({ ...prev, [key]: value }));
  }, []);

  const toggleCalendarStatus = useCallback((status) => {
    setFilters(prev => ({
      ...prev,
      calendarStatuses: prev.calendarStatuses.includes(status)
        ? prev.calendarStatuses.filter(s => s !== status)
        : [...prev.calendarStatuses, status]
    }));
  }, []);

  const clearFilters = useCallback(() => {
    setFilters({
      search: '',
      status: '',
      location: '',
      room: '',
      calendarStatuses: [],
    });
  }, []);

  // === Управление UI ===
  const setViewMode = useCallback((mode) => {
    setUi(prev => ({ ...prev, viewMode: mode, showForm: false }));
  }, []);

  const setCurrentDate = useCallback((date) => {
    setUi(prev => ({ ...prev, currentDate: date }));
  }, []);

  // === Права доступа ===
  const permissions = useMemo(() => ({
    canCreate: ['Admin', 'Researcher'].includes(user?.role),
    canEdit: ['Admin', 'Researcher'].includes(user?.role),
    canDelete: user?.role === 'Admin',
  }), [user?.role]);

  return {
    // Данные
    experiments: filteredExperiments,
    allExperiments: experiments,
    batches,
    equipment,
    rooms,
    loading,

    // Выбранные элементы
    selectedExperiment,
    editingExperiment,

    // Фильтры
    filters,
    setFilter,
    toggleCalendarStatus,
    clearFilters,

    // UI состояния
    ui,
    setViewMode,
    setCurrentDate,

    // Действия с экспериментами
    loadData,
    deleteExperiment,
    updateStatus,
    viewDetails,
    refreshDetails,
    closeDetails,

    // Форма
    openCreateForm,
    openEditForm,
    closeForm,
    onFormSuccess,

    // Комнаты
    openRoomManager,
    closeRoomManager,

    // Утилиты
    getRoomColor,
    permissions,
  };
};

export default useExperiments;
