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

  // Helper: parse container quantity safely (handles object wrappers)
  const getContainerQty = (container) => {
    const raw = container.quantity ?? container.container_quantity;
    return typeof raw === 'object' && raw !== null ? raw.parsedValue : raw;
  };


const handleQuantityUse = useCallback(async (batch) => {
    const input = usageInputs[batch.id] || { quantity: '' };
    const qty = parseFloat(input.quantity);

    if (!qty || qty <= 0) {
      setUsageError(prev => ({ ...prev, [batch.id]: 'Enter quantity' }));
      return;
    }

    const containers = containersMap[batch.id] || [];
    const selectedValue = usageContainer[batch.id];

    // Selection can be empty, a single ID, or comma-separated IDs (sealed group).
    const selectedIds = selectedValue
      ? String(selectedValue).split(',').filter(Boolean)
      : [];

    // --- Single container selected ---
    if (selectedIds.length === 1) {
      const onlyId = selectedIds[0];
      const container = containers.find(c => c.id === onlyId);
      const cQty = getContainerQty(container);

      if (container && qty > cQty + 0.001) {
        setUsageError(prev => ({
          ...prev,
          [batch.id]: `Max in this container: ${cQty}`,
        }));
        return;
      }

      setUsageLoading(prev => ({ ...prev, [batch.id]: true }));
      setUsageError(prev => ({ ...prev, [batch.id]: '' }));

      try {
        await api.useFromContainer(onlyId, {
          quantity: qty, purpose: null, notes: null,
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
      return;
    }

    // --- Group selected (≥2 IDs) OR no selection: auto-distribute ---
    // Group selection narrows the candidate pool to those specific containers;
    // empty selection uses all containers in the batch.
    const candidateContainers = selectedIds.length > 1
      ? containers.filter(c => selectedIds.includes(c.id))
      : containers;

    if (candidateContainers.length > 0) {
      // Sort: opened first, then by sequence_number
      const sorted = [...candidateContainers]
        .filter(c => getContainerQty(c) > 0)
        .sort((a, b) => (b.is_opened ? 1 : 0) - (a.is_opened ? 1 : 0) || a.sequence_number - b.sequence_number);

      const totalAvailable = sorted.reduce((sum, c) => sum + getContainerQty(c), 0);
      if (qty > totalAvailable + 0.001) {
        setUsageError(prev => ({ ...prev, [batch.id]: `Max available: ${totalAvailable.toFixed(2)}` }));
        return;
      }

      // Build distribution plan
      let remaining = qty;
      const plan = []; // [{ containerId, quantity }]
      for (const c of sorted) {
        if (remaining <= 0.001) break;
        const cQty = getContainerQty(c);
        const take = Math.min(remaining, cQty);
        plan.push({ containerId: c.id, quantity: parseFloat(take.toFixed(6)) });
        remaining -= take;
      }

      setUsageLoading(prev => ({ ...prev, [batch.id]: true }));
      setUsageError(prev => ({ ...prev, [batch.id]: '' }));

      try {
        for (const step of plan) {
          await api.useFromContainer(step.containerId, {
            quantity: step.quantity, purpose: null, notes: null,
          });
        }
        const usedFrom = plan.length === 1 ? '' : ` (from ${plan.length} containers)`;
        setUsageSuccess(prev => ({ ...prev, [batch.id]: `−${qty} ${batch.unit}${usedFrom}` }));
        setUsageInputs(prev => ({ ...prev, [batch.id]: { quantity: '' } }));
        loadBatches();
        loadContainers(batch.id);
        onReagentsRefresh?.();
        setTimeout(() => setUsageSuccess(prev => ({ ...prev, [batch.id]: '' })), 2500);
      } catch (err) {
        // Partial usage may have occurred — refresh data
        loadBatches();
        loadContainers(batch.id);
        onReagentsRefresh?.();
        setUsageError(prev => ({ ...prev, [batch.id]: err.message || 'Error during auto-distribution' }));
      } finally {
        setUsageLoading(prev => ({ ...prev, [batch.id]: false }));
      }
      return;
    }

    // --- Legacy path (no containers attached to batch) ---
    const rawAvailable = batch.quantity - (batch.reserved_quantity || 0);
    const available = typeof rawAvailable === 'object' && rawAvailable !== null
      ? rawAvailable.parsedValue
      : rawAvailable;

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
