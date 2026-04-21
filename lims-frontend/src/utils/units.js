// src/utils/units.js
// Умное форматирование количества + конвертация между совместимыми единицами.
//
// База системы совпадает с бэкендом:
//   - Масса: base = 'g', цепочка μg → mg → g → kg
//   - Объём: base = 'mL', цепочка μL → mL → L
// Прочие единицы (pcs, mol, %) — pass-through без конвертации.

// ==================== FACTORS ====================

// Множитель к базовой единице. Напр. 1 kg = 1000 g → factors['kg'] = 1000
const FACTORS = {
  // Mass (base: g)
  kg: 1000, g: 1, mg: 0.001, 'μg': 0.000001, ug: 0.000001,
  // Volume (base: mL)
  L: 1000, l: 1000, mL: 1, ml: 1, 'μL': 0.001, uL: 0.001,
};

const UNIT_TYPE = {
  // Mass
  kg: 'mass', g: 'mass', mg: 'mass', 'μg': 'mass', ug: 'mass',
  // Volume
  L: 'volume', l: 'volume', mL: 'volume', ml: 'volume', 'μL': 'volume', uL: 'volume',
};

const BASE_UNIT = { mass: 'g', volume: 'mL' };

// Список совместимых единиц, упорядоченный от мелких к крупным (для UI dropdown)
const COMPATIBLE_UNITS = {
  mass: ['μg', 'mg', 'g', 'kg'],
  volume: ['μL', 'mL', 'L'],
};

// ==================== SMART NUMBER FORMAT ====================

/** "Умный" форматтер числа: целое — без дроби, дробное — до 2 знаков. */
function formatSmartNumber(value) {
  const rounded = Math.round(value * 100) / 100;
  // Считаем "почти целым" если дробная часть < 0.005
  if (Math.abs(rounded - Math.round(rounded)) < 0.005) {
    return String(Math.round(rounded));
  }
  // Убираем хвостовой ноль: 99.50 → 99.5
  return rounded.toFixed(2).replace(/\.?0+$/, '') || '0';
}

// ==================== PUBLIC API ====================

/**
 * Форматирует количество в самой удобной единице для отображения.
 * @param {number} value — количество в `baseUnit`
 * @param {string} baseUnit — базовая единица (обычно 'g' или 'mL')
 * @returns {string} напр. "100 L", "50 g", "500 mg", "1.5 kg"
 *
 * Примеры:
 *   formatQuantity(100000, 'mL') → "100 L"
 *   formatQuantity(0.5, 'g') → "500 mg"
 *   formatQuantity(5, 'pcs') → "5 pcs" (pass-through)
 *   formatQuantity(null, 'g') → ""
 */
export function formatQuantity(value, baseUnit) {
  if (value == null || Number.isNaN(value)) return '';
  if (value <= 0) return `0 ${baseUnit || ''}`.trim();

  const type = UNIT_TYPE[baseUnit];
  if (!type) {
    // Неизвестная единица — pass-through
    return `${formatSmartNumber(value)} ${baseUnit}`.trim();
  }

  // Нормализуем значение в базовую единицу на случай если base — не базовая
  // (напр. baseUnit='kg' вместо 'g')
  const valueInBase = value * (FACTORS[baseUnit] ?? 1);

  if (type === 'mass') {
    if (valueInBase >= 1000) return `${formatSmartNumber(valueInBase / 1000)} kg`;
    if (valueInBase >= 1) return `${formatSmartNumber(valueInBase)} g`;
    if (valueInBase >= 0.001) return `${formatSmartNumber(valueInBase * 1000)} mg`;
    return `${formatSmartNumber(valueInBase * 1_000_000)} μg`;
  }

  // volume
  if (valueInBase >= 1000) return `${formatSmartNumber(valueInBase / 1000)} L`;
  if (valueInBase >= 1) return `${formatSmartNumber(valueInBase)} mL`;
  return `${formatSmartNumber(valueInBase * 1000)} μL`;
}

/**
 * Возвращает список единиц, совместимых с данной (для выпадающего списка).
 * @param {string} unit — любая единица
 * @returns {string[]} напр. ['μL', 'mL', 'L'] для 'mL'; [] если unit не распознан
 */
export function getCompatibleUnits(unit) {
  const type = UNIT_TYPE[unit];
  return type ? [...COMPATIBLE_UNITS[type]] : [];
}

/**
 * Конвертирует количество между совместимыми единицами.
 * @param {number} value
 * @param {string} from
 * @param {string} to
 * @returns {number|null} результат или null если единицы несовместимы
 *
 * convertQuantity(500, 'mL', 'L') → 0.5
 * convertQuantity(1, 'kg', 'g') → 1000
 * convertQuantity(100, 'g', 'mL') → null (разные типы)
 */
export function convertQuantity(value, from, to) {
  if (from === to) return value;
  const fromFactor = FACTORS[from];
  const toFactor = FACTORS[to];
  if (fromFactor == null || toFactor == null) return null;
  if (UNIT_TYPE[from] !== UNIT_TYPE[to]) return null;
  return (value * fromFactor) / toFactor;
}

/** true если единица — масса или объём (т.е. у неё есть совместимые альтернативы). */
export function isConvertibleUnit(unit) {
  return UNIT_TYPE[unit] != null;
}
