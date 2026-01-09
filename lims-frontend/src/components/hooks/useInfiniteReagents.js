// src/components/hooks/useInfiniteReagents.js
import { useState, useCallback, useRef, useEffect } from 'react';
import { api } from '../../services/api'; // Путь к api может отличаться, проверьте структуру. Если hooks в src/components/hooks, то ../../services/api верно.

export const useInfiniteReagents = (filters = {}) => {
  const [reagents, setReagents] = useState([]);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState(null);
  const [hasMore, setHasMore] = useState(true);
  const [total, setTotal] = useState(0);
  
  const cursorRef = useRef(null);
  const isInitialMount = useRef(true);

  // Формируем строку зависимостей для фильтров, чтобы хук реагировал на любые изменения
  const filterDeps = JSON.stringify(filters);

  // Загрузка первой страницы
  const loadInitial = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      cursorRef.current = null;
      
      const response = await api.getReagentsCursor({
        limit: 20, // Размер страницы
        ...filters
      });
      
      const data = response.data || [];
      setReagents(data);
      setHasMore(response.has_more);
      setTotal(response.total || data.length);
      cursorRef.current = response.next_cursor;
      
    } catch (err) {
      console.error("Load initial failed:", err);
      setError(err.message);
      setReagents([]);
    } finally {
      setLoading(false);
    }
    // eslint-disable-next-line
  }, [filterDeps]);

  // Загрузка следующей порции
  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore || !cursorRef.current) return;
    
    try {
      setLoadingMore(true);
      
      const response = await api.getReagentsCursor({
        limit: 20,
        cursor: cursorRef.current,
        direction: 'next',
        ...filters
      });
      
      const newReagents = response.data || [];
      
      setReagents(prev => {
        // Дедупликация по ID
        const existingIds = new Set(prev.map(r => r.id));
        const uniqueNew = newReagents.filter(r => !existingIds.has(r.id));
        return [...prev, ...uniqueNew];
      });
      
      setHasMore(response.has_more);
      cursorRef.current = response.next_cursor;
      
    } catch (err) {
      setError(err.message);
    } finally {
      setLoadingMore(false);
    }
    // eslint-disable-next-line
  }, [loadingMore, hasMore, filterDeps]);

  // Сброс и перезагрузка при изменении фильтров
  useEffect(() => {
    // Пропускаем первый рендер если нужно, но обычно для фильтров нужно грузить сразу
    // Если isInitialMount нужен для предотвращения двойной загрузки в React 18 StrictMode:
    /* if (isInitialMount.current) {
      isInitialMount.current = false;
      loadInitial();
      return;
    } */
    
    const timer = setTimeout(() => {
      loadInitial();
    }, 300); // Debounce
    
    return () => clearTimeout(timer);
  }, [loadInitial]);

  // Обновление одного реагента в списке (оптимистичное UI)
  const updateReagent = useCallback((updatedReagent) => {
    setReagents(prev => 
      prev.map(r => r.id === updatedReagent.id ? { ...r, ...updatedReagent } : r)
    );
  }, []);

  // Удаление реагента из списка
  const removeReagent = useCallback((reagentId) => {
    setReagents(prev => prev.filter(r => r.id !== reagentId));
    setTotal(prev => Math.max(0, prev - 1));
  }, []);

  // Добавление реагента в начало списка
  const addReagent = useCallback((newReagent) => {
    setReagents(prev => [newReagent, ...prev]);
    setTotal(prev => prev + 1);
  }, []);

  return {
    reagents,
    loading,
    loadingMore,
    error,
    hasMore,
    total,
    loadMore,
    refresh: loadInitial,
    updateReagent,
    removeReagent,
    addReagent,
  };
};

export default useInfiniteReagents;