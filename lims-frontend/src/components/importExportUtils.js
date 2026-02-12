// src/utils/importExportUtils.js
import * as XLSX from 'xlsx';

/**
 * Экспорт данных в Excel
 */
export const exportToExcel = (data, filename = 'export') => {
  const worksheet = XLSX.utils.json_to_sheet(data);
  const workbook = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(workbook, worksheet, 'Sheet1');
  
  // Устанавливаем ширину колонок автоматически
  const maxWidth = 50;
  const colWidths = [];
  const range = XLSX.utils.decode_range(worksheet['!ref']);
  
  for (let C = range.s.c; C <= range.e.c; ++C) {
    let maxLen = 10;
    for (let R = range.s.r; R <= range.e.r; ++R) {
      const cellAddress = XLSX.utils.encode_cell({ r: R, c: C });
      const cell = worksheet[cellAddress];
      if (cell && cell.v) {
        const len = String(cell.v).length;
        if (len > maxLen) maxLen = len;
      }
    }
    colWidths.push({ wch: Math.min(maxLen + 2, maxWidth) });
  }
  worksheet['!cols'] = colWidths;
  
  XLSX.writeFile(workbook, `${filename}.xlsx`);
};

/**
 * Экспорт данных в CSV
 */
export const exportToCSV = (data, filename = 'export') => {
  if (!data || data.length === 0) return;
  
  const headers = Object.keys(data[0]);
  const csvContent = [
    headers.join(','),
    ...data.map(row =>
      headers.map(header => {
        const value = row[header];
        // Обрабатываем значения с запятыми и кавычками
        if (typeof value === 'string' && (value.includes(',') || value.includes('"'))) {
          return `"${value.replace(/"/g, '""')}"`;
        }
        return value || '';
      }).join(',')
    )
  ].join('\n');
  
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);
  
  link.setAttribute('href', url);
  link.setAttribute('download', `${filename}.csv`);
  link.style.visibility = 'hidden';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

/**
 * Экспорт данных в JSON
 */
export const exportToJSON = (data, filename = 'export') => {
  const jsonContent = JSON.stringify(data, null, 2);
  const blob = new Blob([jsonContent], { type: 'application/json' });
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);
  
  link.setAttribute('href', url);
  link.setAttribute('download', `${filename}.json`);
  link.style.visibility = 'hidden';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

/**
 * Валидация данных для импорта
 */
export const validateImportData = (data, requiredFields) => {
  const errors = [];
  
  data.forEach((row, index) => {
    const rowErrors = [];
    
    requiredFields.forEach(field => {
      if (!row[field] || row[field] === '') {
        rowErrors.push(`Отсутствует обязательное поле "${field}"`);
      }
    });
    
    if (rowErrors.length > 0) {
      errors.push({
        row: index + 1,
        errors: rowErrors
      });
    }
  });
  
  return errors;
};

/**
 * Преобразование данных перед импортом
 */
export const transformImportData = (data, transformRules = {}) => {
  return data.map(row => {
    const transformedRow = { ...row };
    
    Object.keys(transformRules).forEach(field => {
      if (transformedRow[field] !== undefined) {
        const rule = transformRules[field];
        
        if (typeof rule === 'function') {
          transformedRow[field] = rule(transformedRow[field]);
        } else if (rule.type === 'date') {
          // Преобразование дат
          const date = new Date(transformedRow[field]);
          transformedRow[field] = date.toISOString();
        } else if (rule.type === 'number') {
          // Преобразование в число
          transformedRow[field] = parseFloat(transformedRow[field]) || 0;
        } else if (rule.type === 'boolean') {
          // Преобразование в boolean
          transformedRow[field] = ['true', '1', 'да', 'yes'].includes(
            String(transformedRow[field]).toLowerCase()
          );
        }
      }
    });
    
    return transformedRow;
  });
};

/**
 * Создание шаблона для импорта
 */
export const createImportTemplate = (fields, format = 'excel') => {
  const templateData = [fields.reduce((acc, field) => {
    acc[field.name] = field.example || '';
    return acc;
  }, {})];
  
  switch (format) {
    case 'excel':
      exportToExcel(templateData, 'import_template');
      break;
    case 'csv':
      exportToCSV(templateData, 'import_template');
      break;
    case 'json':
      exportToJSON(templateData, 'import_template');
      break;
    default:
      console.error('Неподдерживаемый формат шаблона');
  }
};

/**
 * Получение шаблонов для различных сущностей
 */
export const getEntityTemplates = (entityType) => {
  const templates = {
    experiments: [
      { name: 'title', label: 'Название', example: 'Эксперимент 1', required: true },
      { name: 'description', label: 'Описание', example: 'Описание эксперимента', required: false },
      { name: 'experiment_date', label: 'Дата', example: '2024-01-15', required: true },
      { name: 'instructor', label: 'Преподаватель', example: 'Иванов И.И.', required: false },
      { name: 'student_group', label: 'Группа', example: 'ХМ-101', required: false }
    ],
    reagents: [
      { name: 'name', label: 'Название', example: 'Соляная кислота', required: true },
      { name: 'formula', label: 'Формула', example: 'HCl', required: false },
      { name: 'cas_number', label: 'CAS номер', example: '7647-01-0', required: false },
      { name: 'manufacturer', label: 'Производитель', example: 'Sigma-Aldrich', required: false },
      { name: 'description', label: 'Описание', example: 'Соляная кислота, 37%', required: false }
    ],
    equipment: [
      { name: 'name', label: 'Название', example: 'Микроскоп', required: true },
      { name: 'type_', label: 'Тип', example: 'equipment', required: true },
      { name: 'quantity', label: 'Количество', example: '1', required: true },
      { name: 'unit', label: 'Единица измерения', example: 'шт', required: false },
      { name: 'location', label: 'Расположение', example: 'Лаборатория 1', required: false },
      { name: 'description', label: 'Описание', example: 'Оптический микроскоп Olympus', required: false }
    ],
    batches: [
      { name: 'batch_number', label: 'Номер партии', example: 'B2024001', required: true },
      { name: 'reagent_id', label: 'ID реагента', example: '1', required: true },
      { name: 'quantity', label: 'Количество', example: '500', required: true },
      { name: 'unit', label: 'Единица', example: 'мл', required: true },
      { name: 'expiry_date', label: 'Срок годности', example: '2025-12-31', required: false },
      { name: 'supplier', label: 'Поставщик', example: 'Поставщик 1', required: false },
      { name: 'manufacturer', label: 'Производитель', example: 'Sigma-Aldrich', required: false },
      { name: 'received_date', label: 'Дата получения', example: '2024-01-15', required: false },
      { name: 'location', label: 'Расположение', example: 'Шкаф A1', required: false },
      { name: 'notes', label: 'Примечания', example: 'Хранить в темном месте', required: false }
    ]
  };
  
  return templates[entityType] || [];
};

export default {
  exportToExcel,
  exportToCSV,
  exportToJSON,
  validateImportData,
  transformImportData,
  createImportTemplate,
  getEntityTemplates
};
