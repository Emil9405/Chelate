// src/components/experiments/hooks/useExperimentDetails.js
import { useState, useCallback } from 'react';
import { api } from '../../../services/api';

/**
 * Хук для управления деталями эксперимента
 * Инкапсулирует всю логику работы с реагентами, оборудованием и статусами
 */
export const useExperimentDetails = (experimentData, onUpdate) => {
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState('info');
  
  // Форма добавления реагента
  const [showAddReagent, setShowAddReagent] = useState(false);
  const [selectedBatchId, setSelectedBatchId] = useState('');
  const [reagentQuantity, setReagentQuantity] = useState('');
  const [reagentSearch, setReagentSearch] = useState(''); // Поиск по названию
  
  // Форма добавления оборудования
  const [showAddEquipment, setShowAddEquipment] = useState(false);
  const [selectedEquipmentId, setSelectedEquipmentId] = useState('');
  const [equipmentQuantity, setEquipmentQuantity] = useState('1');
  
  // Общие notes для форм
  const [notes, setNotes] = useState('');

  // Извлекаем данные из структуры ответа API
  const experiment = experimentData?.experiment || experimentData;
  const reagents = experimentData?.reagents || [];
  const expEquipment = experimentData?.equipment || [];
  const documents = experimentData?.documents || [];

  // === Работа со статусом ===
  const handleStatusChange = useCallback(async (newStatus) => {
    try {
      setLoading(true);
      
      const currentStatus = experiment?.status;
      
      // Валидация переходов статусов
      const validTransitions = {
        'planned': ['in_progress', 'cancelled'],
        'in_progress': ['completed', 'cancelled'],
        'completed': [], // Нельзя изменить
        'cancelled': [], // Нельзя изменить
      };
      
      if (!validTransitions[currentStatus]?.includes(newStatus)) {
        return { 
          success: false, 
          message: `Cannot change status from ${currentStatus} to ${newStatus}` 
        };
      }
      
      // Используем специализированные endpoints для правильной обработки
      if (newStatus === 'in_progress' && currentStatus === 'planned') {
        await api.startExperiment(experiment.id);
      } else if (newStatus === 'completed' && currentStatus === 'in_progress') {
        await api.completeExperiment(experiment.id);
      } else if (newStatus === 'cancelled') {
        await api.cancelExperiment(experiment.id);
      } else {
        // Fallback на прямое обновление
        await api.updateExperiment(experiment.id, { status: newStatus });
      }
      
      onUpdate?.();
      return { success: true, message: `Status changed to ${newStatus}` };
    } catch (error) {
      console.error('Error changing status:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [experiment?.id, experiment?.status, onUpdate]);

  // === Работа с реагентами ===
  const handleAddReagent = useCallback(async () => {
    if (!selectedBatchId) {
      return { success: false, message: 'Please select a batch' };
    }
    
    if (!reagentQuantity || parseFloat(reagentQuantity) <= 0) {
      return { success: false, message: 'Please enter a valid quantity (> 0)' };
    }

    try {
      setLoading(true);
      await api.addExperimentReagent(experiment.id, {
        batch_id: selectedBatchId,
        quantity_used: parseFloat(reagentQuantity),
        notes: notes || undefined
      });
      
      // Сбрасываем форму
      setShowAddReagent(false);
      setSelectedBatchId('');
      setReagentQuantity('');
      setReagentSearch('');
      setNotes('');
      
      onUpdate?.();
      return { success: true, message: 'Reagent added successfully' };
    } catch (error) {
      console.error('Error adding reagent:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [experiment?.id, selectedBatchId, reagentQuantity, notes, onUpdate]);

  const handleRemoveReagent = useCallback(async (reagentRecordId) => {
    try {
      setLoading(true);
      await api.removeExperimentReagent(experiment.id, reagentRecordId);
      onUpdate?.();
      return { success: true, message: 'Reagent removed' };
    } catch (error) {
      console.error('Error removing reagent:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [experiment?.id, onUpdate]);

  const handleConsumeReagent = useCallback(async (reagentRecordId) => {
    try {
      setLoading(true);
      await api.consumeExperimentReagent(experiment.id, reagentRecordId);
      onUpdate?.();
      return { success: true, message: 'Reagent consumed' };
    } catch (error) {
      console.error('Error consuming reagent:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [experiment?.id, onUpdate]);

  // === Работа с оборудованием ===
  const handleAddEquipment = useCallback(async () => {
    if (!selectedEquipmentId) {
      return { success: false, message: 'Please select equipment' };
    }

    try {
      setLoading(true);
      await api.addExperimentEquipment(experiment.id, {
        equipment_id: selectedEquipmentId,
        quantity_used: parseInt(equipmentQuantity) || 1,
        notes: notes || undefined
      });
      
      // Сбрасываем форму
      setShowAddEquipment(false);
      setSelectedEquipmentId('');
      setEquipmentQuantity('1');
      setNotes('');
      
      onUpdate?.();
      return { success: true, message: 'Equipment added successfully' };
    } catch (error) {
      console.error('Error adding equipment:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [experiment?.id, selectedEquipmentId, equipmentQuantity, notes, onUpdate]);

  const handleRemoveEquipment = useCallback(async (equipmentRecordId) => {
    try {
      setLoading(true);
      await api.removeExperimentEquipment(experiment.id, equipmentRecordId);
      onUpdate?.();
      return { success: true, message: 'Equipment removed' };
    } catch (error) {
      console.error('Error removing equipment:', error);
      return { success: false, message: error.message };
    } finally {
      setLoading(false);
    }
  }, [experiment?.id, onUpdate]);

  // === Сброс форм ===
  const resetReagentForm = useCallback(() => {
    setShowAddReagent(false);
    setSelectedBatchId('');
    setReagentQuantity('');
    setReagentSearch('');
    setNotes('');
  }, []);

  const resetEquipmentForm = useCallback(() => {
    setShowAddEquipment(false);
    setSelectedEquipmentId('');
    setEquipmentQuantity('1');
    setNotes('');
  }, []);

  return {
    // Состояние
    loading,
    activeTab,
    setActiveTab,
    
    // Данные эксперимента
    experiment,
    reagents,
    expEquipment,
    documents,
    
    // Статус
    handleStatusChange,
    
    // Реагенты
    showAddReagent,
    setShowAddReagent,
    selectedBatchId,
    setSelectedBatchId,
    reagentQuantity,
    setReagentQuantity,
    reagentSearch,
    setReagentSearch,
    handleAddReagent,
    handleRemoveReagent,
    handleConsumeReagent,
    resetReagentForm,
    
    // Оборудование
    showAddEquipment,
    setShowAddEquipment,
    selectedEquipmentId,
    setSelectedEquipmentId,
    equipmentQuantity,
    setEquipmentQuantity,
    handleAddEquipment,
    handleRemoveEquipment,
    resetEquipmentForm,
    
    // Общее
    notes,
    setNotes,
  };
};

export default useExperimentDetails;
