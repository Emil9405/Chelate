// hooks/useRooms.js
// Shared hook для загрузки списка комнат
// v2: модульный кэш — один fetch на все компоненты

import { useState, useEffect, useCallback, useRef } from 'react';
import { api } from '../../services/api';

// Модульный кэш: один промис/данные на всё приложение
let _cache = { data: null, promise: null, timestamp: 0 };
const CACHE_TTL = 30_000; // 30 секунд

const fetchRooms = async (onlyAvailable) => {
  const now = Date.now();
  
  // Если данные свежие — возвращаем из кэша
  if (_cache.data && (now - _cache.timestamp) < CACHE_TTL) {
    return _cache.data;
  }
  
  // Если уже идёт запрос — ждём его (deduplicate)
  if (_cache.promise) {
    return _cache.promise;
  }

  _cache.promise = (async () => {
    try {
      const response = onlyAvailable
        ? await api.getAvailableRooms()
        : await api.getRooms();
      const data = response?.data || response;
      const rooms = Array.isArray(data) ? data : [];
      _cache.data = rooms;
      _cache.timestamp = Date.now();
      return rooms;
    } finally {
      _cache.promise = null;
    }
  })();

  return _cache.promise;
};

/**
 * Hook для загрузки списка комнат
 * @param {boolean} onlyAvailable - загрузить только доступные комнаты
 * @returns {{ rooms, loading, error, refresh }}
 */
export const useRooms = (onlyAvailable = false) => {
  const [rooms, setRooms] = useState(_cache.data || []);
  const [loading, setLoading] = useState(!_cache.data);
  const [error, setError] = useState(null);

  const loadRooms = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchRooms(onlyAvailable);
      setRooms(data);
    } catch (err) {
      console.error('Failed to load rooms:', err);
      setError(err.message);
      setRooms([]);
    } finally {
      setLoading(false);
    }
  }, [onlyAvailable]);

  // Принудительный refresh — сбрасывает кэш
  const refresh = useCallback(async () => {
    _cache = { data: null, promise: null, timestamp: 0 };
    await loadRooms();
  }, [loadRooms]);

  useEffect(() => {
    loadRooms();
  }, [loadRooms]);

  return { rooms, loading, error, refresh };
};

export default useRooms;
