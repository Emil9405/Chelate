// components/Reagents/useBatchLogic.js
// Custom hook: batch state + container-aware usage logic
// v2: updated handleLocationSave для поддержки multi-assignment (массовое размещение)

import { useState, useEffect, useCallback } from 'react';
import { api } from '../../services/api';

const useBatchLogic = (reagentId, isExpanded, onReagentsRefresh) => {
  // Batches
  const [batches, setBatches] = useState([]);
  const [loadingBatches, setLoadingBatches] = useState(false);

  // Containers per batch (keyed by batchId)
  const [containersMap, setContainersMap] = useState({});
  const [containersLoading, setContainersLoading] = useState({});

  // Modals
  const [showCreateBatch, setShowCreateBatch] = useState(false);
  const [showEditBatch, setShowEditBatch] = useState(false);
  const [showUsageHistory, setShowUsageHistory] = useState(false);
  const [showPrintModal, setShowPrintModal] = useState(false);
  const [selectedBatch, setSelectedBatch] = useState(null);

  // State for Placement Modal
  // { containers: [...], action: 'place'|'move', batchId: '...' }
  const [containerToPlace, setContainerToPlace] = useState(null);

  // Inline usage
  const [usageInputs, setUsageInputs] = useState({});
  const [usageLoading, setUsageLoading] = useState({});
  const [usageSuccess, setUsageSuccess] = useState({});
  const [usageError, setUsageError] = useState({});
  const [usageContainer, setUsageContainer] = useState({});

  // Load on expand
  useEffect(() => {
    if (isExpanded && batches.length === 0) {
      loadBatches();
    }
  }, [isExpanded]);

  const loadBatches = useCallback(async () => {
    setLoadingBatches(true);
    try {
      const data = await api.getReagentBatches(reagentId);
      setBatches(Array.isArray(data) ? data : (data.data || []));
    } catch (err) {
      console.error('Failed to load batches:', err);
    } finally {
      setLoadingBatches(false);
    }
  }, [reagentId]);

  const loadContainers = useCallback(async (batchId) => {
    setContainersLoading(prev => ({ ...prev, [batchId]: true }));
    try {
      const data = await api.getContainers(batchId);
      setContainersMap(prev => ({
        ...prev,
        [batchId]: data.containers || data || [],
      }));
    } catch (err) {
      console.error('Failed to load containers:', err);
    } finally {
      setContainersLoading(prev => ({ ...prev, [batchId]: false }));
    }
  }, []);

  const splitBatch = useCallback(async (batchId, packSize) => {
    try {
      await api.splitBatchIntoContainers(batchId, packSize);
      await loadContainers(batchId);
      loadBatches();
      return true;
    } catch (err) {
      console.error('Failed to split batch:', err);
      throw err;
    }
  }, [loadContainers, loadBatches]);

  // Триггеры для открытия модального окна выбора локации
  const placeContainer = useCallback((batchId, containersOrSingle) => {
    const arr = Array.isArray(containersOrSingle) ? containersOrSingle : [containersOrSingle];
    setContainerToPlace({ containers: arr, action: 'place', batchId });
  }, []);

  const moveContainer = useCallback((batchId, containersOrSingle) => {
    const arr = Array.isArray(containersOrSingle) ? containersOrSingle : [containersOrSingle];
    setContainerToPlace({ containers: arr, action: 'move', batchId });
  }, []);

  // ===== НОВЫЙ handleLocationSave: поддержка multi-assignment =====
  // assignments: [{ containerIds: ['id1','id2'], positionId: 'pos1' }, ...]
  const handleLocationSave = useCallback(async (assignments) => {
    if (!containerToPlace) return;
    try {
      // Поддерживаем и старый формат (просто positionId строкой), и новый (массив assignments)
      if (typeof assignments === 'string') {
        // Legacy: одна позиция для всех контейнеров
        const positionId = assignments;
        const ids = containerToPlace.containers.map(c => c.id);
        if (containerToPlace.action === 'move') {
          if (ids.length > 1) {
            await api.moveContainersBulk(ids, positionId);
          } else {
            await api.moveContainer(ids[0], positionId);
          }
        } else {
          if (ids.length > 1) {
            await api.placeContainersBulk(ids, positionId);
          } else {
            await api.placeContainer(ids[0], { position_id: positionId });
          }
        }
      } else if (Array.isArray(assignments)) {
        // Новый формат: массив назначений
        for (const { containerIds, positionId } of assignments) {
          if (containerToPlace.action === 'move') {
            if (containerIds.length > 1) {
              await api.moveContainersBulk(containerIds, positionId);
            } else {
              await api.moveContainer(containerIds[0], positionId);
            }
          } else {
            if (containerIds.length > 1) {
              await api.placeContainersBulk(containerIds, positionId);
            } else {
              await api.placeContainer(containerIds[0], { position_id: positionId });
            }
          }
        }
      }
      // Обновляем данные
      await loadContainers(containerToPlace.batchId);
      setContainerToPlace(null);
      loadBatches();
      onReagentsRefresh?.();
    } catch (err) {
      console.error('Failed to update container location:', err);
      throw err; // Пробрасываем ошибку для обработки в модалке
    }
  }, [containerToPlace, loadContainers, loadBatches, onReagentsRefresh]);

  const handleBatchCreated = useCallback(() => {
    setShowCreateBatch(false);
    loadBatches();
    onReagentsRefresh?.();
  }, [loadBatches, onReagentsRefresh]);

  const handleBatchUpdated = useCallback(() => {
    setShowEditBatch(false);
    setSelectedBatch(null);
    loadBatches();
    onReagentsRefresh?.();
  }, [loadBatches, onReagentsRefresh]);

  const handleDeleteBatch = useCallback(async (batch) => {
    if (!window.confirm(`Delete batch "${batch.batch_number}"?`)) return;
    try {
      await api.deleteBatch(batch.reagent_id, batch.id);
      loadBatches();
      onReagentsRefresh?.();
    } catch (err) {
      alert(err.message || 'Failed to delete batch');
    }
  }, [loadBatches, onReagentsRefresh]);

  // ========== Inline usage (container-aware) ==========

  const getUsageInput = (batchId) => usageInputs[batchId] || { quantity: '' };

  const setUsageQuantity = (batchId, value) => {
    setUsageInputs(prev => ({ ...prev, [batchId]: { quantity: value } }));
    setUsageError(prev => ({ ...prev, [batchId]: '' }));
  };

  const handleQuantityUse = useCallback(async (batch) => {
    const input = usageInputs[batch.id] || { quantity: '' };
    const qty = parseFloat(input.quantity);

    if (!qty || qty <= 0) {
      setUsageError(prev => ({ ...prev, [batch.id]: 'Enter quantity' }));
      return;
    }

    const containers = containersMap[batch.id] || [];
    const selectedContainerId = usageContainer[batch.id];

    if (containers.length > 1 && !selectedContainerId) {
      setUsageError(prev => ({ ...prev, [batch.id]: 'Select a container' }));
      return;
    }

    const containerId = selectedContainerId || (containers.length === 1 ? containers[0].id : null);

    if (containerId) {
      const container = containers.find(c => c.id === containerId);
      
      // Защита от объекта quantity
      const rawContainerQty = container.quantity ?? container.container_quantity;
      const parsedContainerQty = typeof rawContainerQty === 'object' && rawContainerQty !== null 
          ? rawContainerQty.parsedValue 
          : rawContainerQty;

      if (container && qty > parsedContainerQty + 0.001) {
        setUsageError(prev => ({
          ...prev,
          [batch.id]: `Max in this container: ${parsedContainerQty}`,
        }));
        return;
      }

      setUsageLoading(prev => ({ ...prev, [batch.id]: true }));
      setUsageError(prev => ({ ...prev, [batch.id]: '' }));

      try {
        await api.useFromContainer(containerId, {
          quantity: qty,
          purpose: null,
          notes: null,
        });
        setUsageSuccess(prev => ({ ...prev, [batch.id]: `−${qty} ${batch.unit}` }));
        setUsageInputs(prev => ({ ...prev, [batch.id]: { quantity: '' } }));
        loadBatches();
        loadContainers(batch.id);
        onReagentsRefresh?.();
        setTimeout(() => setUsageSuccess(prev => ({ ...prev, [batch.id]: '' })), 2500);
      } catch (err) {
        setUsageError(prev => ({ ...prev, [batch.id]: err.message || 'Error' }));
      } finally {
        setUsageLoading(prev => ({ ...prev, [batch.id]: false }));
      }
    } else {
      // Legacy path (без контейнеров)
      const rawAvailable = batch.quantity - (batch.reserved_quantity || 0);
      const available = typeof rawAvailable === 'object' && rawAvailable !== null ? rawAvailable.parsedValue : rawAvailable;
      
      if (qty > available) {
        setUsageError(prev => ({ ...prev, [batch.id]: `Max: ${available}` }));
        return;
      }

      setUsageLoading(prev => ({ ...prev, [batch.id]: true }));
      setUsageError(prev => ({ ...prev, [batch.id]: '' }));

      try {
        await api.useReagent(reagentId, batch.id, { quantity_used: qty });
        setUsageSuccess(prev => ({ ...prev, [batch.id]: `−${qty} ${batch.unit}` }));
        setUsageInputs(prev => ({ ...prev, [batch.id]: { quantity: '' } }));
        loadBatches();
        onReagentsRefresh?.();
        setTimeout(() => setUsageSuccess(prev => ({ ...prev, [batch.id]: '' })), 2500);
      } catch (err) {
        setUsageError(prev => ({ ...prev, [batch.id]: err.message || 'Error' }));
      } finally {
        setUsageLoading(prev => ({ ...prev, [batch.id]: false }));
      }
    }
  }, [reagentId, usageInputs, usageContainer, containersMap, loadBatches, loadContainers, onReagentsRefresh]);

  const adjustQuantityByPack = (batchId, packSize, direction, available) => {
    const current = parseFloat(getUsageInput(batchId).quantity) || 0;
    let newValue;
    if (direction === 'up') {
      newValue = Math.min(current + packSize, available);
    } else {
      newValue = Math.max(current - packSize, 0);
    }
    setUsageQuantity(batchId, newValue > 0 ? newValue.toString() : '');
  };

  return {
    // Data
    batches, loadingBatches, loadBatches,

    // Containers
    containersMap, containersLoading, loadContainers,
    splitBatch, 
    
    // Экспорты для модалки размещения
    placeContainer, moveContainer, 
    containerToPlace, setContainerToPlace, handleLocationSave,

    // Modals
    showCreateBatch, setShowCreateBatch,
    showEditBatch, setShowEditBatch,
    showUsageHistory, setShowUsageHistory,
    showPrintModal, setShowPrintModal,
    selectedBatch, setSelectedBatch,

    // CRUD
    handleBatchCreated, handleBatchUpdated, handleDeleteBatch,

    // Usage
    getUsageInput, setUsageQuantity, handleQuantityUse, adjustQuantityByPack,
    usageLoading, usageSuccess, usageError,
    usageContainer, setUsageContainer,
  };
};

export default useBatchLogic;
