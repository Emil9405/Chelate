// src/components/hooks/useReagents.js
import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { api } from '../../services/api';

/**
 * Hook for managing reagents loading.
 * Supports both classic pagination (page/perPage) and cursor-based.
 *
 * FIXED: Now properly syncs with external filters
 */
export default function useReagents(externalFilters = {}, options = {}) {
  const { useCursor = false, initialPerPage = 20 } = options;

  // Data state
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  // Pagination state
  const [pagination, setPagination] = useState({
    page: 1,
    perPage: initialPerPage,
    total: 0,
    totalPages: 1,
    hasNext: false,
    hasPrev: false,
    nextCursor: null,
  });

  // Sorting state
  const [sorting, setSorting] = useState({
    sortBy: 'created_at',
    sortOrder: 'desc',
  });

  // Internal filters state (for filters.update() usage)
  const [internalFilters, setInternalFilters] = useState({});

  // Merge external and internal filters
  // External filters take precedence (for controlled mode)
  const filters = useMemo(() => ({
    ...internalFilters,
    ...externalFilters,
  }), [internalFilters, externalFilters]);

  // Stable reference for filters to use in dependencies
  const filtersRef = useRef(filters);
  filtersRef.current = filters;

  // Abort controller ref
  const abortRef = useRef(null);

  /**
   * Main data fetching function
   */
  const fetchData = useCallback(async (paramsOverride = {}) => {
    if (abortRef.current) abortRef.current.abort();
    abortRef.current = new AbortController();

    setLoading(true);
    setError(null);

    try {
      const currentFilters = filtersRef.current;

      const queryParams = {
        // Filters - support both 'search' and 'q' for backend compatibility
        search: currentFilters.search || undefined,
        q: currentFilters.search || undefined,
        status: currentFilters.status || undefined,
        manufacturer: currentFilters.manufacturer || undefined,
        stock_status: currentFilters.stock_status || undefined,
        cas_number: currentFilters.cas_number || undefined,
        has_stock: currentFilters.has_stock,
        // Visibility — hidden reagents excluded by default
        include_hidden: currentFilters.include_hidden || undefined,
        // Location filters
        room_id: currentFilters.room_id || undefined,
        zone_id: currentFilters.zone_id || undefined,
        position_id: currentFilters.position_id || undefined,

        // Sorting
        sort_by: sorting.sortBy,
        sort_order: sorting.sortOrder,

        // Pagination
        per_page: pagination.perPage,
        ...paramsOverride
      };

      // If no explicit page/cursor passed, use current from state
      if (!queryParams.page && !queryParams.cursor && !useCursor) {
        queryParams.page = pagination.page;
      }

      const response = await api.getReagents(queryParams, {
        signal: abortRef.current.signal
      });

      let items = [];
      let meta = {};

      if (Array.isArray(response)) {
        items = response;
      } else if (response.data) {
        items = response.data;
        meta = response.meta || response.pagination || {};
      }

      // Update data
      if (useCursor && queryParams.cursor) {
        setData(prev => [...prev, ...items]);
      } else {
        setData(items);
      }

      // Update pagination metadata
      setPagination(prev => ({
        ...prev,
        page: meta.current_page || meta.page || queryParams.page || 1,
        total: meta.total_records || meta.total_count || meta.total || 0,
        totalPages: meta.total_pages || 1,
        hasNext: !!meta.next_cursor || (meta.current_page < meta.total_pages) || meta.has_next || false,
        hasPrev: meta.has_prev || (meta.current_page > 1) || false,
        nextCursor: meta.next_cursor || null,
      }));

    } catch (err) {
      if (err.name !== 'AbortError') {
        console.error('Failed to fetch reagents:', err);
        setError(err.message || 'Failed to load data');
      }
    } finally {
      if (abortRef.current && !abortRef.current.signal.aborted) {
        setLoading(false);
      }
    }
  }, [sorting, pagination.page, pagination.perPage, useCursor]);

  // === PAGINATION ACTIONS ===

  const goNext = useCallback(() => {
    if (useCursor && pagination.nextCursor) {
      fetchData({ cursor: pagination.nextCursor });
    } else if (!useCursor && pagination.page < pagination.totalPages) {
      setPagination(prev => ({ ...prev, page: prev.page + 1 }));
    }
  }, [pagination, useCursor, fetchData]);

  const goPrev = useCallback(() => {
    if (!useCursor && pagination.page > 1) {
      setPagination(prev => ({ ...prev, page: prev.page - 1 }));
    }
  }, [pagination, useCursor]);

  const setPerPage = useCallback((count) => {
    const newPerPage = parseInt(count, 10);
    setPagination(prev => ({
      ...prev,
      perPage: newPerPage,
      page: 1
    }));
  }, []);

  const goToPage = useCallback((page) => {
    if (!useCursor && page >= 1 && page <= pagination.totalPages) {
      setPagination(prev => ({ ...prev, page }));
    }
  }, [pagination.totalPages, useCursor]);

  // === SORTING ACTIONS ===

  const setSort = useCallback((field) => {
    setSorting(prev => {
      if (prev.sortBy === field) {
        return { ...prev, sortOrder: prev.sortOrder === 'asc' ? 'desc' : 'asc' };
      }
      return { sortBy: field, sortOrder: 'desc' };
    });
    setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
  }, []);

  const setSortOrder = useCallback((order) => {
    const normalizedOrder = order.toLowerCase() === 'asc' ? 'asc' : 'desc';
    setSorting(prev => ({ ...prev, sortOrder: normalizedOrder }));
    setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
  }, []);

  const setSortFull = useCallback((field, order) => {
    const normalizedOrder = (order || 'desc').toLowerCase() === 'asc' ? 'asc' : 'desc';
    setSorting({ sortBy: field, sortOrder: normalizedOrder });
    setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
  }, []);

  // === FILTER ACTIONS ===

  const updateFilter = useCallback((key, value) => {
    setInternalFilters(prev => ({ ...prev, [key]: value }));
    setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
  }, []);

  const setFiltersAll = useCallback((newFilters) => {
    setInternalFilters(newFilters);
    setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
  }, []);

  const clearFilters = useCallback(() => {
    setInternalFilters({});
    setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
  }, []);

  // === DATA ACTIONS ===

  const removeItem = useCallback((id) => {
    setData(prev => prev.filter(item => item.id !== id));
    setPagination(prev => ({ ...prev, total: Math.max(0, prev.total - 1) }));
  }, []);

  // === AUTO-FETCH ON DEPENDENCY CHANGE ===

  // Stringify filters for stable dependency comparison
  const filtersKey = JSON.stringify(filters);

  // Reset page to 1 when filters change
  const prevFiltersKey = useRef(filtersKey);
  useEffect(() => {
    if (prevFiltersKey.current !== filtersKey) {
      prevFiltersKey.current = filtersKey;
      setPagination(prev => ({ ...prev, page: 1, nextCursor: null }));
    }
  }, [filtersKey]);

  useEffect(() => {
    if (!useCursor || (useCursor && data.length === 0)) {
      fetchData();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pagination.page, pagination.perPage, sorting.sortBy, sorting.sortOrder, filtersKey]);

  return {
    data,
    loading,
    error,
    pagination: {
      ...pagination,
      goNext,
      goPrev,
      goToPage,
      setPerPage,
    },
    sorting: {
      ...sorting,
      setSort,
      setSortOrder,
      setSortFull,
    },
    filters: {
      values: filters,
      update: updateFilter,
      setAll: setFiltersAll,
      clear: clearFilters,
    },
    refresh: () => fetchData(),
    actions: {
      removeItem,
      fetchData
    }
  };
}