// components/AdvancedFilters.js
// Продвинутая система фильтров для LIMS v4.0
// Поддержка: вложенных групп AND/OR, всех операторов, диапазонов, массивов

import React, { useState, useCallback, useMemo, useEffect } from 'react';

// ==================== КОНСТАНТЫ ====================

// Все поддерживаемые операторы
export const OPERATORS = {
  // Базовые
  eq: { label: 'Равно', symbol: '=', types: ['string', 'number', 'date', 'enum'] },
  neq: { label: 'Не равно', symbol: '≠', types: ['string', 'number', 'date', 'enum'] },
  // Числовые
  gt: { label: 'Больше', symbol: '>', types: ['number', 'date'] },
  gte: { label: 'Больше или равно', symbol: '≥', types: ['number', 'date'] },
  lt: { label: 'Меньше', symbol: '<', types: ['number', 'date'] },
  lte: { label: 'Меньше или равно', symbol: '≤', types: ['number', 'date'] },
  // Строковые
  like: { label: 'Содержит', symbol: '∋', types: ['string'] },
  starts_with: { label: 'Начинается с', symbol: '^', types: ['string'] },
  ends_with: { label: 'Заканчивается на', symbol: '$', types: ['string'] },
  // Массивы
  in: { label: 'В списке', symbol: '∈', types: ['string', 'number', 'enum'] },
  not_in: { label: 'Не в списке', symbol: '∉', types: ['string', 'number', 'enum'] },
  // Null checks
  is_null: { label: 'Пусто', symbol: '∅', types: ['string', 'number', 'date', 'enum'], noValue: true },
  is_not_null: { label: 'Не пусто', symbol: '!∅', types: ['string', 'number', 'date', 'enum'], noValue: true },
  // Диапазоны
  between: { label: 'Между', symbol: '↔', types: ['number', 'date'], isRange: true },
  not_between: { label: 'Не между', symbol: '!↔', types: ['number', 'date'], isRange: true },
};

// Типы полей для разных таблиц
export const FIELD_CONFIGS = {
  batches: {
    id: { label: 'ID', type: 'string' },
    reagent_id: { label: 'ID реагента', type: 'string' },
    reagent_name: { label: 'Название реагента', type: 'string' },
    batch_number: { label: 'Номер партии', type: 'string' },
    cat_number: { label: 'Каталожный номер', type: 'string' },
    quantity: { label: 'Количество', type: 'number' },
    original_quantity: { label: 'Исходное количество', type: 'number' },
    reserved_quantity: { label: 'Зарезервировано', type: 'number' },
    unit: { label: 'Единица', type: 'enum', options: ['г', 'мл', 'шт', 'кг', 'л'] },
    expiry_date: { label: 'Срок годности', type: 'date' },
    supplier: { label: 'Поставщик', type: 'string' },
    manufacturer: { label: 'Производитель', type: 'string' },
    status: { label: 'Статус', type: 'enum', options: ['available', 'reserved', 'expired', 'depleted'] },
    location: { label: 'Местоположение', type: 'string' },
    days_until_expiry: { label: 'Дней до истечения', type: 'number' },
  },
  reagents: {
    id: { label: 'ID', type: 'string' },
    name: { label: 'Название', type: 'string' },
    formula: { label: 'Формула', type: 'string' },
    cas_number: { label: 'CAS номер', type: 'string' },
    manufacturer: { label: 'Производитель', type: 'string' },
    physical_state: { label: 'Агрегатное состояние', type: 'enum', options: ['solid', 'liquid', 'gas'] },
    status: { label: 'Статус', type: 'enum', options: ['active', 'inactive', 'discontinued'] },
  },
  experiments: {
    id: { label: 'ID', type: 'string' },
    title: { label: 'Название', type: 'string' },
    description: { label: 'Описание', type: 'string' },
    experiment_date: { label: 'Дата эксперимента', type: 'date' },
    instructor: { label: 'Инструктор', type: 'string' },
    student_group: { label: 'Группа', type: 'string' },
    status: { label: 'Статус', type: 'enum', options: ['planned', 'in_progress', 'completed', 'cancelled'] },
    experiment_type: { label: 'Тип', type: 'enum', options: ['educational', 'research'] },
  },
  equipment: {
    id: { label: 'ID', type: 'string' },
    name: { label: 'Название', type: 'string' },
    serial_number: { label: 'Серийный номер', type: 'string' },
    status: { label: 'Статус', type: 'enum', options: ['available', 'in_use', 'maintenance', 'broken'] },
    location: { label: 'Местоположение', type: 'string' },
    last_maintenance: { label: 'Последнее ТО', type: 'date' },
    next_maintenance: { label: 'Следующее ТО', type: 'date' },
  },
};

// Стили
const styles = {
  container: {
    backgroundColor: '#f8fafc',
    borderRadius: '12px',
    padding: '16px',
    marginBottom: '16px',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '16px',
  },
  title: {
    fontSize: '16px',
    fontWeight: '600',
    color: '#1e293b',
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  group: {
    backgroundColor: '#fff',
    borderRadius: '8px',
    padding: '12px',
    marginBottom: '8px',
    border: '1px solid #e2e8f0',
  },
  groupHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    marginBottom: '12px',
  },
  groupTypeSelect: {
    padding: '4px 12px',
    borderRadius: '6px',
    border: '1px solid #cbd5e1',
    fontSize: '13px',
    fontWeight: '500',
    cursor: 'pointer',
    backgroundColor: '#f1f5f9',
  },
  filterRow: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    marginBottom: '8px',
    padding: '8px',
    backgroundColor: '#f8fafc',
    borderRadius: '6px',
  },
  select: {
    padding: '8px 12px',
    borderRadius: '6px',
    border: '1px solid #cbd5e1',
    fontSize: '14px',
    backgroundColor: '#fff',
    cursor: 'pointer',
    minWidth: '120px',
  },
  input: {
    padding: '8px 12px',
    borderRadius: '6px',
    border: '1px solid #cbd5e1',
    fontSize: '14px',
    flex: 1,
    minWidth: '100px',
  },
  button: {
    padding: '8px 12px',
    borderRadius: '6px',
    border: 'none',
    fontSize: '14px',
    fontWeight: '500',
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
  },
  primaryButton: {
    backgroundColor: '#3b82f6',
    color: '#fff',
  },
  secondaryButton: {
    backgroundColor: '#e2e8f0',
    color: '#475569',
  },
  dangerButton: {
    backgroundColor: '#fee2e2',
    color: '#dc2626',
  },
  iconButton: {
    padding: '6px',
    borderRadius: '4px',
    border: 'none',
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'transparent',
  },
  nestedGroup: {
    marginLeft: '24px',
    borderLeft: '2px solid #cbd5e1',
    paddingLeft: '12px',
  },
  rangeInputs: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    flex: 1,
  },
  tagInput: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '4px',
    padding: '4px 8px',
    borderRadius: '6px',
    border: '1px solid #cbd5e1',
    backgroundColor: '#fff',
    minHeight: '36px',
    alignItems: 'center',
    flex: 1,
  },
  tag: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    padding: '2px 8px',
    borderRadius: '4px',
    backgroundColor: '#dbeafe',
    color: '#1e40af',
    fontSize: '13px',
  },
  tagRemove: {
    cursor: 'pointer',
    marginLeft: '4px',
    opacity: 0.7,
  },
  presets: {
    display: 'flex',
    gap: '8px',
    marginBottom: '12px',
    flexWrap: 'wrap',
  },
  presetButton: {
    padding: '6px 12px',
    borderRadius: '16px',
    border: '1px solid #e2e8f0',
    backgroundColor: '#fff',
    fontSize: '13px',
    cursor: 'pointer',
    transition: 'all 0.2s',
  },
  presetButtonActive: {
    backgroundColor: '#3b82f6',
    color: '#fff',
    borderColor: '#3b82f6',
  },
  actions: {
    display: 'flex',
    gap: '8px',
    marginTop: '16px',
    justifyContent: 'flex-end',
  },
  badge: {
    padding: '2px 6px',
    borderRadius: '4px',
    fontSize: '11px',
    fontWeight: '600',
  },
};

// ==================== УТИЛИТЫ ====================

// Генерация уникального ID
const generateId = () => `f_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

// Создание пустого фильтра
const createEmptyFilter = (fieldConfig) => {
  const firstField = Object.keys(fieldConfig)[0];
  const firstFieldType = fieldConfig[firstField]?.type || 'string';
  const validOperators = Object.entries(OPERATORS)
    .filter(([_, op]) => op.types.includes(firstFieldType))
    .map(([key]) => key);

  return {
    id: generateId(),
    field: firstField,
    operator: validOperators[0] || 'eq',
    value: '',
    enabled: true,
  };
};

// Создание пустой группы
const createEmptyGroup = (groupType = 'AND', fieldConfig) => ({
  id: generateId(),
  group: groupType,
  items: [createEmptyFilter(fieldConfig)],
});

// Преобразование в JSON для API
export const filtersToJson = (group) => {
  const processItem = (item) => {
    if (item.group) {
      return {
        group: item.group,
        items: item.items.filter(i => i.enabled !== false).map(processItem),
      };
    }
    
    const filter = {
      field: item.field,
      operator: item.operator,
    };

    // Добавляем value только если оператор требует его
    const opConfig = OPERATORS[item.operator];
    if (!opConfig?.noValue) {
      if (opConfig?.isRange) {
        filter.value = { from: item.valueFrom || item.value, to: item.valueTo };
      } else if (item.operator === 'in' || item.operator === 'not_in') {
        filter.value = Array.isArray(item.value) ? item.value : [item.value].filter(Boolean);
      } else {
        filter.value = item.value;
      }
    }

    return filter;
  };

  return processItem(group);
};

// Преобразование из JSON
export const jsonToFilters = (json, fieldConfig) => {
  const processItem = (item) => {
    if (item.group) {
      return {
        id: generateId(),
        group: item.group,
        items: item.items.map(i => processItem(i)),
      };
    }

    return {
      id: generateId(),
      field: item.field,
      operator: item.operator,
      value: item.value?.from || item.value,
      valueFrom: item.value?.from,
      valueTo: item.value?.to,
      enabled: true,
    };
  };

  return processItem(json);
};

// ==================== КОМПОНЕНТЫ ====================

const FilterValueInput = ({ filter, fieldConfig, onChange }) => {
  const fieldType = fieldConfig[filter.field]?.type || 'string';
  const fieldOptions = fieldConfig[filter.field]?.options || [];
  const opConfig = OPERATORS[filter.operator];

  // Хук всегда вверху — теперь всё по правилам React
  const isArrayOperator = filter.operator === 'in' || filter.operator === 'not_in';
  const values = isArrayOperator && Array.isArray(filter.value) ? filter.value : [];
  const [inputValue, setInputValue] = useState('');

  // Сбрасываем временное значение при смене оператора
  useEffect(() => {
    if (isArrayOperator) {
      setInputValue('');
    }
  }, [filter.operator, isArrayOperator]);

  const addValue = () => {
    if (inputValue.trim() && !values.includes(inputValue.trim())) {
      onChange({ ...filter, value: [...values, inputValue.trim()] });
      setInputValue('');
    }
  };

  const removeValue = (idx) => {
    onChange({ ...filter, value: values.filter((_, i) => i !== idx) });
  };

  if (opConfig?.noValue) {
    return <span style={{ color: '#64748b', fontSize: '13px', fontStyle: 'italic' }}>—</span>;
  }

  if (opConfig?.isRange) {
    const inputType = fieldType === 'date' ? 'date' : 'number';
    return (
      <div style={styles.rangeInputs}>
        <input
          type={inputType}
          style={{ ...styles.input, minWidth: '80px' }}
          placeholder="От"
          value={filter.valueFrom || ''}
          onChange={(e) => onChange({ ...filter, valueFrom: e.target.value })}
        />
        <span style={{ color: '#64748b' }}>—</span>
        <input
          type={inputType}
          style={{ ...styles.input, minWidth: '80px' }}
          placeholder="До"
          value={filter.valueTo || ''}
          onChange={(e) => onChange({ ...filter, valueTo: e.target.value })}
        />
      </div>
    );
  }

  if (isArrayOperator) {
    if (fieldType === 'enum' && fieldOptions.length > 0) {
      return (
        <div style={styles.tagInput}>
          {values.map((v, idx) => (
            <span key={idx} style={styles.tag}>
              {v}
              <span style={styles.tagRemove} onClick={() => removeValue(idx)}>×</span>
            </span>
          ))}
          <select
            style={{ ...styles.select, minWidth: '80px', border: 'none', backgroundColor: 'transparent' }}
            value=""
            onChange={(e) => {
              if (e.target.value && !values.includes(e.target.value)) {
                onChange({ ...filter, value: [...values, e.target.value] });
              }
            }}
          >
            <option value="">+ Добавить</option>
            {fieldOptions.filter(opt => !values.includes(opt)).map(opt => (
              <option key={opt} value={opt}>{opt}</option>
            ))}
          </select>
        </div>
      );
    }

    return (
      <div style={styles.tagInput}>
        {values.map((v, idx) => (
          <span key={idx} style={styles.tag}>
            {v}
            <span style={styles.tagRemove} onClick={() => removeValue(idx)}>×</span>
          </span>
        ))}
        <input
          type="text"
          style={{ border: 'none', outline: 'none', flex: 1, minWidth: '60px', fontSize: '14px' }}
          placeholder="Введите и нажмите Enter"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              addValue();
            }
          }}
        />
      </div>
    );
  }

  // Остальные типы (enum, date, number, string) — без изменений
  if (fieldType === 'enum' && fieldOptions.length > 0) {
    return (
      <select style={styles.select} value={filter.value || ''} onChange={(e) => onChange({ ...filter, value: e.target.value })}>
        <option value="">Выберите...</option>
        {fieldOptions.map(opt => <option key={opt} value={opt}>{opt}</option>)}
      </select>
    );
  }

  if (fieldType === 'date') {
    return <input type="date" style={styles.input} value={filter.value || ''} onChange={(e) => onChange({ ...filter, value: e.target.value })} />;
  }

  if (fieldType === 'number') {
    return <input type="number" style={styles.input} placeholder="Значение" value={filter.value || ''} onChange={(e) => onChange({ ...filter, value: e.target.value })} />;
  }

  return <input type="text" style={styles.input} placeholder="Значение" value={filter.value || ''} onChange={(e) => onChange({ ...filter, value: e.target.value })} />;
};

// Компонент одного фильтра
const FilterRow = ({ filter, fieldConfig, onChange, onRemove, onToggle }) => {
  const fieldType = fieldConfig[filter.field]?.type || 'string';
  
  // Получить доступные операторы для типа поля
  const availableOperators = useMemo(() => {
    return Object.entries(OPERATORS)
      .filter(([_, op]) => op.types.includes(fieldType))
      .map(([key, op]) => ({ key, ...op }));
  }, [fieldType]);

  // При смене поля - сбросить оператор если он не поддерживается
  const handleFieldChange = useCallback((newField) => {
    const newFieldType = fieldConfig[newField]?.type || 'string';
    const currentOpValid = OPERATORS[filter.operator]?.types.includes(newFieldType);
    
    const newOperator = currentOpValid 
      ? filter.operator 
      : Object.entries(OPERATORS).find(([_, op]) => op.types.includes(newFieldType))?.[0] || 'eq';

    onChange({
      ...filter,
      field: newField,
      operator: newOperator,
      value: '',
      valueFrom: '',
      valueTo: '',
    });
  }, [filter, fieldConfig, onChange]);

  return (
    <div style={{ ...styles.filterRow, opacity: filter.enabled ? 1 : 0.5 }}>
      {/* Чекбокс включения */}
      <input
        type="checkbox"
        checked={filter.enabled}
        onChange={() => onToggle(filter.id)}
        style={{ cursor: 'pointer' }}
      />

      {/* Выбор поля */}
      <select
        style={styles.select}
        value={filter.field}
        onChange={(e) => handleFieldChange(e.target.value)}
      >
        {Object.entries(fieldConfig).map(([key, config]) => (
          <option key={key} value={key}>{config.label}</option>
        ))}
      </select>

      {/* Выбор оператора */}
      <select
        style={{ ...styles.select, minWidth: '140px' }}
        value={filter.operator}
        onChange={(e) => onChange({ ...filter, operator: e.target.value, value: '', valueFrom: '', valueTo: '' })}
      >
        {availableOperators.map(op => (
          <option key={op.key} value={op.key}>{op.symbol} {op.label}</option>
        ))}
      </select>

      {/* Ввод значения */}
      <FilterValueInput filter={filter} fieldConfig={fieldConfig} onChange={onChange} />

      {/* Кнопка удаления */}
      <button
        style={{ ...styles.iconButton, color: '#dc2626' }}
        onClick={() => onRemove(filter.id)}
        title="Удалить фильтр"
      >
        🗑️
      </button>
    </div>
  );
};

// Компонент группы фильтров
const FilterGroupComponent = ({
  group,
  fieldConfig,
  onChange,
  onRemove,
  depth = 0,
  maxDepth = 3,
}) => {
  // Обновить элемент группы
  const updateItem = useCallback((itemId, newItem) => {
    const newItems = group.items.map(item => 
      (item.id === itemId) ? newItem : item
    );
    onChange({ ...group, items: newItems });
  }, [group, onChange]);

  // Удалить элемент
  const removeItem = useCallback((itemId) => {
    const newItems = group.items.filter(item => item.id !== itemId);
    if (newItems.length === 0) {
      // Если группа пуста - добавляем пустой фильтр
      newItems.push(createEmptyFilter(fieldConfig));
    }
    onChange({ ...group, items: newItems });
  }, [group, fieldConfig, onChange]);

  // Переключить enabled
  const toggleItem = useCallback((itemId) => {
    const newItems = group.items.map(item => 
      (item.id === itemId) ? { ...item, enabled: !item.enabled } : item
    );
    onChange({ ...group, items: newItems });
  }, [group, onChange]);

  // Добавить фильтр
  const addFilter = useCallback(() => {
    onChange({
      ...group,
      items: [...group.items, createEmptyFilter(fieldConfig)],
    });
  }, [group, fieldConfig, onChange]);

  // Добавить вложенную группу
  const addGroup = useCallback(() => {
    if (depth >= maxDepth) return;
    onChange({
      ...group,
      items: [...group.items, createEmptyGroup(group.group === 'AND' ? 'OR' : 'AND', fieldConfig)],
    });
  }, [group, fieldConfig, depth, maxDepth, onChange]);

  return (
    <div style={{ ...styles.group, ...(depth > 0 ? styles.nestedGroup : {}) }}>
      <div style={styles.groupHeader}>
        <select
          style={{
            ...styles.groupTypeSelect,
            backgroundColor: group.group === 'AND' ? '#dbeafe' : '#fef3c7',
            color: group.group === 'AND' ? '#1e40af' : '#92400e',
          }}
          value={group.group}
          onChange={(e) => onChange({ ...group, group: e.target.value })}
        >
          <option value="AND">AND (Все условия)</option>
          <option value="OR">OR (Любое условие)</option>
        </select>

        <span style={{ flex: 1 }} />

        {depth > 0 && (
          <button
            style={{ ...styles.iconButton, color: '#dc2626' }}
            onClick={onRemove}
            title="Удалить группу"
          >
            🗑️
          </button>
        )}
      </div>

      {/* Элементы группы */}
      {group.items.map((item) => (
        item.group ? (
          <FilterGroupComponent
            key={item.id}
            group={item}
            fieldConfig={fieldConfig}
            onChange={(newGroup) => updateItem(item.id, newGroup)}
            onRemove={() => removeItem(item.id)}
            depth={depth + 1}
            maxDepth={maxDepth}
          />
        ) : (
          <FilterRow
            key={item.id}
            filter={item}
            fieldConfig={fieldConfig}
            onChange={(newFilter) => updateItem(item.id, newFilter)}
            onRemove={() => removeItem(item.id)}
            onToggle={toggleItem}
          />
        )
      ))}

      {/* Кнопки добавления */}
      <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
        <button
          style={{ ...styles.button, ...styles.secondaryButton }}
          onClick={addFilter}
        >
          ➕ Добавить фильтр
        </button>
        {depth < maxDepth && (
          <button
            style={{ ...styles.button, ...styles.secondaryButton }}
            onClick={addGroup}
          >
            📁 Добавить группу
          </button>
        )}
      </div>
    </div>
  );
};

// ==================== ГЛАВНЫЙ КОМПОНЕНТ ====================

export const AdvancedFilters = ({
  // Таблица для которой строятся фильтры
  table = 'batches',
  // Начальные фильтры (JSON или объект)
  initialFilters = null,
  // Callback при изменении фильтров
  onChange,
  // Callback при применении фильтров
  onApply,
  // Callback при сбросе
  onReset,
  // Пресеты фильтров
  presets = [],
  // Показывать кнопку применения
  showApplyButton = true,
  // Максимальная вложенность групп
  maxDepth = 3,
  // Свернуть по умолчанию
  defaultCollapsed = false,
}) => {
  const fieldConfig = FIELD_CONFIGS[table] || FIELD_CONFIGS.batches;
  
  const [filters, setFilters] = useState(() => {
    if (initialFilters) {
      return typeof initialFilters === 'string' 
        ? jsonToFilters(JSON.parse(initialFilters), fieldConfig)
        : jsonToFilters(initialFilters, fieldConfig);
    }
    return createEmptyGroup('AND', fieldConfig);
  });

  const [collapsed, setCollapsed] = useState(defaultCollapsed);
  const [activePreset, setActivePreset] = useState(null);

  // Подсчет активных фильтров
  const activeFiltersCount = useMemo(() => {
    const countFilters = (group) => {
      return group.items.reduce((acc, item) => {
        if (item.group) {
          return acc + countFilters(item);
        }
        return acc + (item.enabled ? 1 : 0);
      }, 0);
    };
    return countFilters(filters);
  }, [filters]);

  // Обработчик изменения фильтров
  const handleChange = useCallback((newFilters) => {
    setFilters(newFilters);
    setActivePreset(null);
    if (onChange) {
      onChange(filtersToJson(newFilters));
    }
  }, [onChange]);

  // Применение пресета
  const applyPreset = useCallback((preset) => {
    const newFilters = jsonToFilters(preset.filters, fieldConfig);
    setFilters(newFilters);
    setActivePreset(preset.id);
    if (onChange) {
      onChange(filtersToJson(newFilters));
    }
  }, [fieldConfig, onChange]);

  // Сброс фильтров
  const resetFilters = useCallback(() => {
    const newFilters = createEmptyGroup('AND', fieldConfig);
    setFilters(newFilters);
    setActivePreset(null);
    if (onReset) {
      onReset();
    }
    if (onChange) {
      onChange(null);
    }
  }, [fieldConfig, onChange, onReset]);

  // Применение фильтров
  const applyFilters = useCallback(() => {
    if (onApply) {
      onApply(filtersToJson(filters));
    }
  }, [filters, onApply]);

  return (
    <div style={styles.container}>
      {/* Заголовок */}
      <div style={styles.header}>
        <div style={styles.title}>
          <span>🔍</span>
          <span>Фильтры</span>
          {activeFiltersCount > 0 && (
            <span style={{
              ...styles.badge,
              backgroundColor: '#dbeafe',
              color: '#1e40af',
            }}>
              {activeFiltersCount}
            </span>
          )}
        </div>
        <button
          style={{ ...styles.iconButton, fontSize: '18px' }}
          onClick={() => setCollapsed(!collapsed)}
        >
          {collapsed ? '▼' : '▲'}
        </button>
      </div>

      {!collapsed && (
        <>
          {/* Пресеты */}
          {presets.length > 0 && (
            <div style={styles.presets}>
              {presets.map(preset => (
                <button
                  key={preset.id}
                  style={{
                    ...styles.presetButton,
                    ...(activePreset === preset.id ? styles.presetButtonActive : {}),
                  }}
                  onClick={() => applyPreset(preset)}
                >
                  {preset.icon && <span>{preset.icon}</span>}
                  {preset.label}
                </button>
              ))}
            </div>
          )}

          {/* Группа фильтров */}
          <FilterGroupComponent
            group={filters}
            fieldConfig={fieldConfig}
            onChange={handleChange}
            maxDepth={maxDepth}
          />

          {/* Кнопки действий */}
          <div style={styles.actions}>
            <button
              style={{ ...styles.button, ...styles.secondaryButton }}
              onClick={resetFilters}
            >
              🔄 Сбросить
            </button>
            {showApplyButton && (
              <button
                style={{ ...styles.button, ...styles.primaryButton }}
                onClick={applyFilters}
              >
                ✓ Применить
              </button>
            )}
          </div>
        </>
      )}
    </div>
  );
};

// ==================== ГОТОВЫЕ ПРЕСЕТЫ ====================

export const BATCH_PRESETS = [
  {
    id: 'low_stock',
    label: 'Низкий запас',
    icon: '📉',
    filters: {
      group: 'AND',
      items: [
        { field: 'quantity', operator: 'lte', value: 10 },
        { field: 'status', operator: 'eq', value: 'available' },
      ],
    },
  },
  {
    id: 'expiring_soon',
    label: 'Истекает скоро',
    icon: '⏰',
    filters: {
      group: 'AND',
      items: [
        { field: 'days_until_expiry', operator: 'between', value: { from: 0, to: 30 } },
        { field: 'status', operator: 'neq', value: 'expired' },
      ],
    },
  },
  {
    id: 'expired',
    label: 'Просроченные',
    icon: '⚠️',
    filters: {
      group: 'OR',
      items: [
        { field: 'status', operator: 'eq', value: 'expired' },
        { field: 'days_until_expiry', operator: 'lt', value: 0 },
      ],
    },
  },
  {
    id: 'available',
    label: 'Доступные',
    icon: '✅',
    filters: {
      group: 'AND',
      items: [
        { field: 'status', operator: 'eq', value: 'available' },
        { field: 'quantity', operator: 'gt', value: 0 },
      ],
    },
  },
];

export const EXPERIMENT_PRESETS = [
  {
    id: 'planned',
    label: 'Запланированные',
    icon: '📅',
    filters: {
      group: 'AND',
      items: [
        { field: 'status', operator: 'eq', value: 'planned' },
      ],
    },
  },
  {
    id: 'in_progress',
    label: 'В процессе',
    icon: '🔬',
    filters: {
      group: 'AND',
      items: [
        { field: 'status', operator: 'eq', value: 'in_progress' },
      ],
    },
  },
  {
    id: 'educational',
    label: 'Учебные',
    icon: '📚',
    filters: {
      group: 'AND',
      items: [
        { field: 'experiment_type', operator: 'eq', value: 'educational' },
      ],
    },
  },
];

// ==================== ХЕЛПЕРЫ ДЛЯ API ====================

// Преобразовать фильтры в query string для GET запросов
export const filtersToQueryString = (filters) => {
  if (!filters) return '';
  return `filters=${encodeURIComponent(JSON.stringify(filters))}`;
};

// Парсинг фильтров из query string
export const parseFiltersFromQuery = (queryString) => {
  const params = new URLSearchParams(queryString);
  const filtersJson = params.get('filters');
  if (filtersJson) {
    try {
      return JSON.parse(decodeURIComponent(filtersJson));
    } catch (e) {
      console.error('Failed to parse filters:', e);
    }
  }
  return null;
};

export default AdvancedFilters;