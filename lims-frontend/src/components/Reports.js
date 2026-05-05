// components/Reports.js
// Экспорт CSV + XLSX через бэкенд (POST /api/v1/reports/export, body.format = "csv" | "xlsx").
// Фронт валидирует Content-Type — если пришёл text/html, выбрасывает ошибку,
// чтобы юзер не сохранил HTML под видом отчёта.

import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { api } from '../services/api';
import ErrorMessage from './ErrorMessage';
import Loading from './Loading';
import Table from './Table';
import Badge from './Badge';
import Button from './Button';
import Select from './Select';
import Input from './Input';

// ==================== CONSTANTS ====================

const ALLOWED_SORT_FIELDS = new Set([
  'id', 'reagent_id', 'reagent_name', 'batch_number', 'cat_number',
  'quantity', 'original_quantity', 'reserved_quantity', 'unit',
  'expiry_date', 'supplier', 'manufacturer', 'received_date',
  'status', 'created_at', 'updated_at', 'days_until_expiry',
  'expiration_status',
  'container_count', 'opened_count', 'placed_count', 'unplaced_count',
  'location_summary', 'room_names',
]);

const BATCH_STATUSES = ['available', 'low_stock', 'reserved', 'expired', 'depleted'];

// API base — у тебя в проекте обычно через relative paths и proxy/CRA.
// Если в api.js есть getBaseURL — лучше использовать его.
const API_BASE = '/api/v1';

// ==================== UTILITIES ====================

const useDebounce = (value, delay) => {
  const [debouncedValue, setDebouncedValue] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedValue(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debouncedValue;
};

/** Достаёт JWT так же, как это делает api.js (LIMS использует localStorage). */
const getAuthToken = () => {
  try {
    return localStorage.getItem('lims_token') || localStorage.getItem('token') || '';
  } catch {
    return '';
  }
};

/** Извлекает имя файла из Content-Disposition или генерирует фолбэк. */
const filenameFromHeaders = (response, fallback) => {
  const cd = response.headers.get('Content-Disposition') || '';
  const match = cd.match(/filename\*?=(?:UTF-8'')?"?([^"';]+)"?/i);
  return match ? decodeURIComponent(match[1]) : fallback;
};

/** Скачивает blob как файл. */
const downloadBlob = (blob, filename) => {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
};

// ==================== COMPONENT ====================

const Reports = ({ user }) => {
  const [activeReport, setActiveReport] = useState('low_stock');
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState('');
  const [reportData, setReportData] = useState([]);
  const [reportMetadata, setReportMetadata] = useState(null);

  const [threshold, setThreshold] = useState(10);
  const [expiringDays, setExpiringDays] = useState(30);

  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(50);
  const [totalPages, setTotalPages] = useState(1);
  const [totalItems, setTotalItems] = useState(0);

  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState('created_at');
  const [sortOrder, setSortOrder] = useState('DESC');

  const debouncedSearch = useDebounce(searchTerm, 300);
  const abortControllerRef = useRef(null);

  const [availableColumns, setAvailableColumns] = useState([]);
  const [availableFields, setAvailableFields] = useState([]);
  const [visibleColumns, setVisibleColumns] = useState([]);
  const [activeFilters, setActiveFilters] = useState([]);

  const [showFiltersPanel, setShowFiltersPanel] = useState(false);
  const [showColumnsPanel, setShowColumnsPanel] = useState(false);

  const [newFilter, setNewFilter] = useState({ field: '', operator: '', value: '' });

  const reportPresets = [
    { value: 'low_stock', label: '📉 Low Stock', description: 'Batches with quantity below threshold' },
    { value: 'expiring_soon', label: '⏰ Expiring Soon', description: 'Batches expiring within specified days' },
    { value: 'expired', label: '❌ Expired', description: 'Batches that have expired' },
    { value: 'all_batches', label: '📋 All Batches', description: 'Active batches (excludes depleted)' },
    { value: 'depleted', label: '📦 Depleted', description: 'Fully consumed batches — archive / history' },
    { value: 'unplaced', label: '📍 Unplaced', description: 'Batches with containers not assigned to a storage position' },
    { value: 'custom', label: '🔧 Custom', description: 'Build your own report with filters' },
  ];

  const operatorLabels = {
    eq: '= equals', ne: '≠ not equals', gt: '> greater than',
    gte: '≥ greater or equal', lt: '< less than', lte: '≤ less or equal',
    like: '~ contains', in: '∈ in list', not_in: '∉ not in list',
    is_null: '∅ is empty', is_not_null: '✓ is not empty',
  };

  const operatorShortLabels = {
    eq: '=', ne: '≠', gt: '>', gte: '≥', lt: '<', lte: '≤',
    like: '~', in: '∈', not_in: '∉', is_null: '∅', is_not_null: '✓',
  };

  // ==================== DEFAULT DATA ====================

  const defaultFields = useMemo(() => [
    { field: 'status', label: 'Status', data_type: 'enum', operators: ['eq', 'ne', 'in'], values: BATCH_STATUSES },
    { field: 'quantity', label: 'Quantity', data_type: 'number', operators: ['eq', 'gt', 'gte', 'lt', 'lte'], values: null },
    { field: 'expiry_date', label: 'Expiry Date', data_type: 'date', operators: ['eq', 'gt', 'lt', 'is_null'], values: null },
    { field: 'location_summary', label: 'Location', data_type: 'text', operators: ['eq', 'like', 'is_null', 'is_not_null'], values: null },
    { field: 'room_names', label: 'Rooms', data_type: 'text', operators: ['like', 'is_null', 'is_not_null'], values: null },
    { field: 'container_count', label: 'Containers (total)', data_type: 'number', operators: ['eq', 'gt', 'gte', 'lt', 'lte'], values: null },
    { field: 'opened_count', label: 'Containers (opened)', data_type: 'number', operators: ['eq', 'gt', 'gte', 'lt', 'lte'], values: null },
    { field: 'placed_count', label: 'Containers (placed)', data_type: 'number', operators: ['eq', 'gt', 'gte', 'lt', 'lte'], values: null },
    { field: 'unplaced_count', label: 'Containers (unplaced)', data_type: 'number', operators: ['eq', 'gt', 'gte', 'lt', 'lte'], values: null },
    { field: 'supplier', label: 'Supplier', data_type: 'text', operators: ['eq', 'like'], values: null },
    { field: 'days_until_expiry', label: 'Days Until Expiry', data_type: 'number', operators: ['gt', 'gte', 'lt', 'lte'], values: null },
    { field: 'manufacturer', label: 'Manufacturer', data_type: 'text', operators: ['eq', 'like'], values: null },
    { field: 'reagent_name', label: 'Reagent Name', data_type: 'text', operators: ['eq', 'like'], values: null },
  ], []);

  const defaultColumns = useMemo(() => [
    { field: 'reagent_name', label: 'Reagent', data_type: 'text', visible: true, sortable: true },
    { field: 'batch_number', label: 'Batch #', data_type: 'text', visible: true, sortable: true },
    { field: 'quantity', label: 'Quantity', data_type: 'number', visible: true, sortable: true },
    { field: 'unit', label: 'Unit', data_type: 'text', visible: false, sortable: false },
    { field: 'expiry_date', label: 'Expiry Date', data_type: 'date', visible: true, sortable: true },
    { field: 'days_until_expiry', label: 'Days Left', data_type: 'number', visible: true, sortable: true },
    { field: 'status', label: 'Status', data_type: 'enum', visible: true, sortable: true },
    { field: 'room_names', label: 'Rooms', data_type: 'text', visible: true, sortable: true },
    { field: 'location_summary', label: 'Location', data_type: 'text', visible: false, sortable: true },
    { field: 'container_count', label: 'Containers', data_type: 'number', visible: true, sortable: true },
    { field: 'opened_count', label: 'Opened', data_type: 'number', visible: false, sortable: true },
    { field: 'placed_count', label: 'Placed', data_type: 'number', visible: false, sortable: true },
    { field: 'unplaced_count', label: 'Unplaced', data_type: 'number', visible: false, sortable: true },
    { field: 'supplier', label: 'Supplier', data_type: 'text', visible: false, sortable: true },
    { field: 'manufacturer', label: 'Manufacturer', data_type: 'text', visible: false, sortable: true },
    { field: 'cat_number', label: 'Cat #', data_type: 'text', visible: false, sortable: true },
    { field: 'received_date', label: 'Received', data_type: 'date', visible: false, sortable: true },
    { field: 'notes', label: 'Notes', data_type: 'text', visible: false, sortable: false },
  ], []);

  // ==================== LOAD METADATA ====================

  useEffect(() => {
    const loadMetadata = async () => {
      try {
        const fieldsResponse = await api.getReportFields();
        const fields = fieldsResponse?.data || fieldsResponse || [];
        if (Array.isArray(fields) && fields.length > 0) {
          const statusField = fields.find(f => f.field === 'status');
          if (statusField?.values && !statusField.values.includes('low_stock')) {
            statusField.values = BATCH_STATUSES;
          }
          setAvailableFields(fields);
        } else {
          setAvailableFields(defaultFields);
        }
        setAvailableColumns(defaultColumns);
        setVisibleColumns(defaultColumns.filter(c => c.visible !== false).map(c => c.field));
      } catch (err) {
        console.error('Failed to load report metadata:', err);
        setAvailableFields(defaultFields);
        setAvailableColumns(defaultColumns);
        setVisibleColumns(defaultColumns.filter(c => c.visible).map(c => c.field));
      }
    };
    loadMetadata();
  }, [defaultFields, defaultColumns]);

  // ==================== LOAD REPORT ====================

  const loadReport = useCallback(async () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    abortControllerRef.current = new AbortController();

    try {
      setLoading(true);
      setError('');

      const presetParams = {};
      if (activeReport === 'low_stock') presetParams.threshold = threshold;
      else if (activeReport === 'expiring_soon') presetParams.days = expiringDays;

      const requestBody = {
        preset: activeReport,
        preset_params: presetParams,
        page,
        per_page: perPage,
        sort_by: sortBy,
        sort_order: sortOrder,
        search: debouncedSearch || undefined,
        columns: visibleColumns,
        filters: activeFilters.map(f => {
          const fieldDef = availableFields.find(af => af.field === f.field);
          let value = f.value;
          if (fieldDef?.data_type === 'number' && typeof value === 'string') {
            const num = parseFloat(value);
            if (!isNaN(num)) value = num;
          }
          return { field: f.field, operator: f.operator, value };
        }),
      };

      const response = await api.generateReport(requestBody);

      if (abortControllerRef.current?.signal.aborted) return;

      if (response && response.data) {
        setReportData(response.data);
        setReportMetadata(response.metadata);
        if (response.pagination) {
          setTotalPages(response.pagination.total_pages || 1);
          setTotalItems(response.pagination.total || 0);
        }
      } else if (Array.isArray(response)) {
        setReportData(response);
        setTotalItems(response.length);
      } else {
        setReportData([]);
      }
    } catch (err) {
      if (err.name === 'AbortError') return;
      console.error('Failed to load report:', err);
      setError(err.message || 'Failed to load report');
      setReportData([]);
    } finally {
      setLoading(false);
    }
  }, [activeReport, threshold, expiringDays, page, perPage, sortBy, sortOrder, debouncedSearch, visibleColumns, activeFilters, availableFields]);

  useEffect(() => {
    loadReport();
    return () => {
      if (abortControllerRef.current) abortControllerRef.current.abort();
    };
  }, [loadReport]);

  useEffect(() => {
    setPage(1);
  }, [activeReport, debouncedSearch, threshold, expiringDays, activeFilters]);

  // ==================== HANDLERS ====================

  const addFilter = useCallback(() => {
    if (!newFilter.field || !newFilter.operator) return;
    const filterToAdd = {
      ...newFilter,
      id: Date.now(),
      value: ['is_null', 'is_not_null'].includes(newFilter.operator) ? true : newFilter.value,
    };
    setActiveFilters(prev => [...prev, filterToAdd]);
    setNewFilter({ field: '', operator: '', value: '' });
  }, [newFilter]);

  const removeFilter = useCallback((id) => {
    setActiveFilters(prev => prev.filter(f => f.id !== id));
  }, []);

  const toggleColumn = useCallback((field) => {
    setVisibleColumns(prev =>
      prev.includes(field) ? prev.filter(f => f !== field) : [...prev, field]
    );
  }, []);

  const handleSort = useCallback((field) => {
    if (!ALLOWED_SORT_FIELDS.has(field)) {
      console.warn(`Sort field "${field}" not allowed`);
      return;
    }
    if (sortBy === field) {
      setSortOrder(prev => prev === 'ASC' ? 'DESC' : 'ASC');
    } else {
      setSortBy(field);
      setSortOrder('ASC');
    }
  }, [sortBy]);

  // ==================== EXPORT ====================
  //
  // Один универсальный обработчик: format = "csv" | "xlsx".
  // Запрос идёт на /api/v1/reports/export, бэкенд формирует файл и отдаёт
  // как поток с правильными заголовками. Фронт ВАЛИДИРУЕТ Content-Type —
  // если пришёл text/html, значит роут не отработал и нас бы ждал
  // HTML-файл под расширением .csv/.xlsx (старый баг).
  // ==================================================

  const exportReport = useCallback(async (format) => {
    if (exporting) return;
    if (!reportData || reportData.length === 0) {
      alert('No data to export');
      return;
    }

    const expectedMime = format === 'xlsx'
      ? 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
      : 'text/csv';
    const fallbackName = `report_${activeReport}_${new Date().toISOString().split('T')[0]}.${format}`;

    setExporting(true);
    try {
      const presetParams = {};
      if (activeReport === 'low_stock') presetParams.threshold = threshold;
      if (activeReport === 'expiring_soon') presetParams.days = expiringDays;

      const body = {
        preset: activeReport,
        preset_params: presetParams,
        format,
        sort_by: sortBy,
        sort_order: sortOrder,
        search: debouncedSearch || undefined,
        columns: visibleColumns,
        filters: activeFilters.map(f => ({
          field: f.field,
          operator: f.operator,
          value: f.value,
        })),
      };

      const token = getAuthToken();
      const response = await fetch(`${API_BASE}/reports/export`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        // Пробуем достать текст ошибки (часто JSON, иногда text)
        let detail = '';
        try { detail = await response.text(); } catch (_) {}
        throw new Error(`Export failed: ${response.status} ${detail.slice(0, 200)}`);
      }

      // Защита от подмены ответа на HTML (SPA-fallback и т.п.)
      const ct = (response.headers.get('Content-Type') || '').toLowerCase();
      if (ct.includes('text/html')) {
        throw new Error('Server returned HTML instead of file. Check auth/routing.');
      }
      if (!ct.includes(expectedMime) && !ct.includes('octet-stream')) {
        console.warn(`Unexpected Content-Type: ${ct}, expected ${expectedMime}`);
      }

      const blob = await response.blob();
      const filename = filenameFromHeaders(response, fallbackName);
      downloadBlob(blob, filename);
    } catch (err) {
      console.error(`${format.toUpperCase()} export failed:`, err);
      alert('Export failed: ' + (err.message || 'Unknown error'));
    } finally {
      setExporting(false);
    }
  }, [exporting, reportData, activeReport, threshold, expiringDays, sortBy, sortOrder, debouncedSearch, visibleColumns, activeFilters]);

  // ==================== RENDER HELPERS ====================

  const getStatusVariant = useCallback((item) => {
    if (item.days_until_expiry !== null && item.days_until_expiry !== undefined) {
      if (item.days_until_expiry < 0) return 'danger';
      if (item.days_until_expiry < 7) return 'danger';
      if (item.days_until_expiry < 30) return 'warning';
    }
    if (item.status === 'expired') return 'danger';
    if (item.status === 'depleted') return 'secondary';
    if (item.status === 'reserved') return 'warning';
    if (item.status === 'low_stock') return 'warning';
    return 'success';
  }, []);

  const renderCell = useCallback((item, field) => {
    const value = item[field];

    switch (field) {
      case 'quantity':
        return (
          <span style={{
            color: value < 10 ? '#e53e3e' : value < 20 ? '#dd6b20' : 'inherit',
            fontWeight: value < 10 ? 'bold' : 'normal'
          }}>
            {value} {item.unit || ''}
          </span>
        );
      case 'expiry_date': {
        if (!value) return <span style={{ color: '#a0aec0' }}>—</span>;
        const date = new Date(value);
        const days = item.days_until_expiry;
        return (
          <div>
            <div>{date.toLocaleDateString()}</div>
            {days !== null && days !== undefined && (
              <small style={{
                color: days < 0 ? '#e53e3e' : days < 7 ? '#e53e3e' : days < 30 ? '#dd6b20' : '#718096'
              }}>
                {days < 0 ? `${Math.abs(days)}d ago` : `${days}d left`}
              </small>
            )}
          </div>
        );
      }
      case 'days_until_expiry':
        if (value === null || value === undefined) return '—';
        return (
          <span style={{
            color: value < 0 ? '#e53e3e' : value < 7 ? '#e53e3e' : value < 30 ? '#dd6b20' : 'inherit',
            fontWeight: value < 7 ? 'bold' : 'normal'
          }}>
            {value}
          </span>
        );
      case 'status':
        return (
          <Badge variant={getStatusVariant(item)}>
            {item.expiration_status === 'expired' ? 'Expired' :
             item.expiration_status === 'critical' ? 'Critical' :
             item.expiration_status === 'warning' ? 'Warning' :
             value || 'Available'}
          </Badge>
        );
      case 'received_date':
        return value ? new Date(value).toLocaleDateString() : '—';

      case 'room_names': {
        if (!value) return <span style={{ color: '#a0aec0' }}>—</span>;
        const rooms = value.split(',').map(r => r.trim()).filter(Boolean);
        return (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
            {rooms.map(name => (
              <span key={name} style={{
                padding: '2px 8px', fontSize: '11px', fontWeight: 600,
                color: '#2d3748', backgroundColor: '#ebf4ff',
                border: '1px solid #90cdf4', borderRadius: '10px',
              }}>
                {name}
              </span>
            ))}
          </div>
        );
      }
      case 'location_summary':
        if (!value) return <span style={{ color: '#a0aec0' }}>—</span>;
        return (
          <span style={{ fontSize: '0.8rem', color: '#4a5568' }} title={value}>
            {value.length > 60 ? value.slice(0, 60) + '…' : value}
          </span>
        );
      case 'container_count':
      case 'opened_count':
      case 'placed_count':
      case 'unplaced_count': {
        const n = value ?? 0;
        if (n === 0 && field !== 'container_count') {
          return <span style={{ color: '#a0aec0' }}>0</span>;
        }
        const color = field === 'unplaced_count' && n > 0 ? '#e53e3e'
                    : field === 'opened_count' && n > 0 ? '#dd6b20'
                    : 'inherit';
        return <span style={{ color, fontWeight: n > 0 ? 600 : 400 }}>{n}</span>;
      }

      default:
        return value || '—';
    }
  }, [getStatusVariant]);

  const tableColumns = useMemo(() =>
    visibleColumns.map(field => {
      const col = availableColumns.find(c => c.field === field) || { field, label: field };
      return {
        key: field,
        label: col.label,
        sortable: col.sortable !== false && ALLOWED_SORT_FIELDS.has(field),
        render: (item) => renderCell(item, field),
      };
    }),
    [visibleColumns, availableColumns, renderCell]
  );

  const currentFieldConfig = useMemo(() =>
    availableFields.find(f => f.field === newFilter.field),
    [availableFields, newFilter.field]
  );

  // ==================== RENDER ====================

  return (
    <div style={{
      padding: '1.5rem', marginTop: '70px',
      minHeight: 'calc(100vh - 70px)', backgroundColor: '#f7fafc'
    }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        marginBottom: '1rem', backgroundColor: '#fff',
        padding: '1rem 1.5rem', borderRadius: '8px',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}>
        <h2 style={{ margin: 0, fontSize: '1.5rem', color: '#2d3748' }}>📊 Reports</h2>
        <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
          <Button
            variant={showFiltersPanel ? 'primary' : 'secondary'}
            onClick={() => setShowFiltersPanel(!showFiltersPanel)}
          >
            🔍 Filters {activeFilters.length > 0 && `(${activeFilters.length})`}
          </Button>
          <Button
            variant={showColumnsPanel ? 'primary' : 'secondary'}
            onClick={() => setShowColumnsPanel(!showColumnsPanel)}
          >
            📋 Columns
          </Button>
          <Button onClick={loadReport} disabled={loading}>🔄 Refresh</Button>
          <Button
            onClick={() => exportReport('csv')}
            disabled={loading || exporting || !reportData.length}
            title="Export as CSV"
          >
            {exporting ? '⏳' : '📥'} CSV
          </Button>
          <Button
            onClick={() => exportReport('xlsx')}
            disabled={loading || exporting || !reportData.length}
            title="Export as Excel workbook"
          >
            {exporting ? '⏳' : '📊'} XLSX
          </Button>
        </div>
      </div>

      <div style={{
        display: 'flex', gap: '0.5rem', marginBottom: '1rem', flexWrap: 'wrap',
        backgroundColor: '#fff', padding: '1rem', borderRadius: '8px',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}>
        {reportPresets.map(preset => (
          <Button
            key={preset.value}
            variant={activeReport === preset.value ? 'primary' : 'secondary'}
            onClick={() => {
              setActiveReport(preset.value);
              if (preset.value === 'custom') setShowFiltersPanel(true);
            }}
            title={preset.description}
            style={{ minWidth: '120px' }}
          >
            {preset.label}
          </Button>
        ))}
      </div>

      {showFiltersPanel && (
        <div style={{
          backgroundColor: '#fff', padding: '1rem', borderRadius: '8px',
          boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '1rem'
        }}>
          <h4 style={{ margin: '0 0 1rem 0', color: '#4a5568' }}>🔍 Filter Builder</h4>

          <div style={{ display: 'flex', gap: '0.75rem', flexWrap: 'wrap', alignItems: 'flex-end', marginBottom: '1rem' }}>
            <div style={{ minWidth: '180px' }}>
              <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>
                Field ({availableFields.length} available)
              </label>
              <select
                value={newFilter.field}
                onChange={(e) => setNewFilter({ field: e.target.value, operator: '', value: '' })}
                style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid #e2e8f0', fontSize: '0.875rem', backgroundColor: '#fff' }}
              >
                <option value="">Select field...</option>
                {availableFields.map(f => (
                  <option key={f.field} value={f.field}>{f.label}</option>
                ))}
              </select>
            </div>

            {newFilter.field && currentFieldConfig && (
              <div style={{ minWidth: '160px' }}>
                <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>Operator</label>
                <select
                  value={newFilter.operator}
                  onChange={(e) => setNewFilter(prev => ({ ...prev, operator: e.target.value, value: '' }))}
                  style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid #e2e8f0', fontSize: '0.875rem', backgroundColor: '#fff' }}
                >
                  <option value="">Select...</option>
                  {currentFieldConfig.operators.map(op => (
                    <option key={op} value={op}>{operatorLabels[op] || op}</option>
                  ))}
                </select>
              </div>
            )}

            {newFilter.field && newFilter.operator && !['is_null', 'is_not_null'].includes(newFilter.operator) && (
              <div style={{ minWidth: '200px', flex: 1 }}>
                <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>Value</label>
                {currentFieldConfig?.values ? (
                  <select
                    value={newFilter.value}
                    onChange={(e) => setNewFilter(prev => ({ ...prev, value: e.target.value }))}
                    style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid #e2e8f0', fontSize: '0.875rem', backgroundColor: '#fff' }}
                  >
                    <option value="">Select value...</option>
                    {currentFieldConfig.values.map(v => (
                      <option key={v} value={v}>{v}</option>
                    ))}
                  </select>
                ) : (
                  <input
                    type={currentFieldConfig?.data_type === 'number' ? 'number' : 'text'}
                    value={newFilter.value}
                    onChange={(e) => setNewFilter(prev => ({ ...prev, value: e.target.value }))}
                    placeholder="Enter value..."
                    style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid #e2e8f0', fontSize: '0.875rem' }}
                  />
                )}
              </div>
            )}

            <button
              onClick={addFilter}
              disabled={!newFilter.field || !newFilter.operator || (!['is_null', 'is_not_null'].includes(newFilter.operator) && !newFilter.value)}
              style={{
                height: '38px', padding: '0.5rem 1rem',
                backgroundColor: (!newFilter.field || !newFilter.operator || (!['is_null', 'is_not_null'].includes(newFilter.operator) && !newFilter.value)) ? '#e2e8f0' : '#667eea',
                color: (!newFilter.field || !newFilter.operator || (!['is_null', 'is_not_null'].includes(newFilter.operator) && !newFilter.value)) ? '#a0aec0' : '#fff',
                border: 'none', borderRadius: '4px',
                cursor: (!newFilter.field || !newFilter.operator || (!['is_null', 'is_not_null'].includes(newFilter.operator) && !newFilter.value)) ? 'not-allowed' : 'pointer',
                fontWeight: '500'
              }}
            >
              ➕ Add
            </button>
          </div>

          {activeFilters.length > 0 && (
            <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', alignItems: 'center' }}>
              <span style={{ fontSize: '0.875rem', fontWeight: '600', color: '#4a5568' }}>Active:</span>
              {activeFilters.map(filter => {
                const fieldDef = availableFields.find(f => f.field === filter.field);
                return (
                  <span key={filter.id} style={{
                    display: 'inline-flex', alignItems: 'center', gap: '0.25rem',
                    padding: '0.25rem 0.5rem', backgroundColor: '#edf2f7',
                    borderRadius: '4px', fontSize: '0.875rem'
                  }}>
                    <strong>{fieldDef?.label || filter.field}</strong>
                    <span style={{ color: '#667eea' }}>{operatorShortLabels[filter.operator]}</span>
                    {!['is_null', 'is_not_null'].includes(filter.operator) && (
                      <span style={{ color: '#38a169' }}>"{filter.value}"</span>
                    )}
                    <button
                      onClick={() => removeFilter(filter.id)}
                      aria-label="Remove filter"
                      style={{
                        background: 'none', border: 'none', cursor: 'pointer',
                        padding: '0 0.25rem', color: '#e53e3e', fontWeight: 'bold'
                      }}
                    >×</button>
                  </span>
                );
              })}
              <Button variant="link" onClick={() => setActiveFilters([])}>Clear all</Button>
            </div>
          )}
        </div>
      )}

      {showColumnsPanel && (
        <div style={{
          backgroundColor: '#fff', padding: '1rem', borderRadius: '8px',
          boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '1rem'
        }}>
          <h4 style={{ margin: '0 0 1rem 0', color: '#4a5568' }}>📋 Select Columns</h4>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
            {availableColumns.map(col => (
              <label key={col.field} style={{
                display: 'flex', alignItems: 'center', gap: '0.375rem',
                padding: '0.375rem 0.75rem',
                backgroundColor: visibleColumns.includes(col.field) ? '#ebf4ff' : '#f7fafc',
                border: `1px solid ${visibleColumns.includes(col.field) ? '#667eea' : '#e2e8f0'}`,
                borderRadius: '4px', cursor: 'pointer', fontSize: '0.875rem',
                transition: 'all 0.15s'
              }}>
                <input
                  type="checkbox"
                  checked={visibleColumns.includes(col.field)}
                  onChange={() => toggleColumn(col.field)}
                  style={{ cursor: 'pointer' }}
                />
                {col.label}
              </label>
            ))}
          </div>
        </div>
      )}

      <div style={{
        display: 'flex', gap: '1rem', marginBottom: '1rem', flexWrap: 'wrap',
        alignItems: 'flex-end', backgroundColor: '#fff', padding: '1rem',
        borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}>
        <div style={{ flex: 1, minWidth: '200px' }}>
          <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>Search</label>
          <Input
            type="text"
            placeholder="Search reagents, batches..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>

        {activeReport === 'low_stock' && (
          <div style={{ width: '140px' }}>
            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>Threshold</label>
            <Input
              type="number" min="1" value={threshold}
              onChange={(e) => setThreshold(parseInt(e.target.value) || 10)}
            />
          </div>
        )}

        {activeReport === 'expiring_soon' && (
          <div style={{ width: '140px' }}>
            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>Days ahead</label>
            <Input
              type="number" min="1" value={expiringDays}
              onChange={(e) => setExpiringDays(parseInt(e.target.value) || 30)}
            />
          </div>
        )}

        <div style={{ width: '100px' }}>
          <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '600', color: '#718096', marginBottom: '0.25rem' }}>Per page</label>
          <Select
            value={perPage}
            onChange={(e) => setPerPage(parseInt(e.target.value))}
            options={[
              { value: 25, label: '25' },
              { value: 50, label: '50' },
              { value: 100, label: '100' },
              { value: 200, label: '200' },
            ]}
          />
        </div>
      </div>

      {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}

      {reportMetadata && (
        <div style={{
          backgroundColor: '#edf2f7', padding: '0.75rem 1rem', borderRadius: '8px',
          marginBottom: '1rem', fontSize: '0.875rem',
          display: 'flex', justifyContent: 'space-between', alignItems: 'center'
        }}>
          <div>
            <strong>{reportMetadata.name}</strong>
            {reportMetadata.description && (
              <span style={{ color: '#718096', marginLeft: '0.5rem' }}>— {reportMetadata.description}</span>
            )}
          </div>
          <span style={{ fontWeight: '600', color: '#4a5568' }}>{totalItems} items</span>
        </div>
      )}

      {loading && <Loading message="Loading report..." />}

      {!loading && (
        <div style={{
          backgroundColor: '#fff', borderRadius: '8px',
          boxShadow: '0 1px 3px rgba(0,0,0,0.1)', overflow: 'hidden'
        }}>
          <Table
            data={reportData}
            columns={tableColumns}
            onSort={handleSort}
            sortBy={sortBy}
            sortOrder={sortOrder}
            emptyMessage={`No ${reportPresets.find(p => p.value === activeReport)?.label || 'items'} found`}
          />
        </div>
      )}

      {!loading && totalPages > 1 && (
        <div style={{
          display: 'flex', justifyContent: 'center', alignItems: 'center',
          gap: '0.5rem', marginTop: '1rem', padding: '1rem',
          backgroundColor: '#fff', borderRadius: '8px',
          boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
        }}>
          <Button variant="secondary" onClick={() => setPage(1)} disabled={page <= 1}>⏮️</Button>
          <Button variant="secondary" onClick={() => setPage(p => Math.max(1, p - 1))} disabled={page <= 1}>◀️</Button>
          <span style={{ padding: '0 1rem', fontWeight: '500' }}>Page {page} of {totalPages}</span>
          <Button variant="secondary" onClick={() => setPage(p => Math.min(totalPages, p + 1))} disabled={page >= totalPages}>▶️</Button>
          <Button variant="secondary" onClick={() => setPage(totalPages)} disabled={page >= totalPages}>⏭️</Button>
        </div>
      )}
    </div>
  );
};

export default Reports;
