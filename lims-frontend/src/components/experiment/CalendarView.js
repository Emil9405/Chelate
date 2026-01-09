// src/components/experiments/CalendarView.js
import React, { useMemo, useCallback } from 'react';
import styles from './CalendarView.module.css';

// === Константы ===
const MONTH_NAMES = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December'
];

const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

const MAX_VISIBLE_EXPERIMENTS = 3;

// === Утилиты ===
const formatLocalDate = (date) => {
  const y = date.getFullYear();
  const m = (date.getMonth() + 1).toString().padStart(2, '0');
  const d = date.getDate().toString().padStart(2, '0');
  return `${y}-${m}-${d}`;
};

const getStatusColor = (status) => {
  switch (status) {
    case 'completed': return '#10b981';
    case 'in_progress': return '#f59e0b';
    case 'cancelled': return '#ef4444';
    default: return '#667eea';
  }
};

// === Подкомпоненты ===

const CalendarHeader = ({ month, year, onPrevious, onNext, onToday }) => (
  <div className={styles.header}>
    <button onClick={onPrevious} className={styles.navButton}>
      <i className="fas fa-chevron-left"></i>
    </button>
    
    <div className={styles.monthTitle}>
      {MONTH_NAMES[month]} {year}
    </div>

    <div className={styles.headerRight}>
      <button onClick={onToday} className={styles.todayButton}>
        Today
      </button>
      <button onClick={onNext} className={styles.navButton}>
        <i className="fas fa-chevron-right"></i>
      </button>
    </div>
  </div>
);

const DayNamesRow = () => (
  <div className={styles.dayNamesRow}>
    {DAY_NAMES.map(day => (
      <div key={day} className={styles.dayName}>
        {day}
      </div>
    ))}
  </div>
);

const ExperimentItem = ({ experiment, onClick, getRoomColor }) => {
  const color = getRoomColor?.(experiment.location) || getStatusColor(experiment.status);
  
  return (
    <div
      onClick={() => onClick(experiment.id)}
      className={styles.experimentItem}
      style={{
        backgroundColor: `${color}20`,
        borderLeftColor: color,
      }}
      title={experiment.title}
    >
      {experiment.title}
    </div>
  );
};

const DayCell = ({ 
  day, 
  isToday, 
  isEmpty, 
  experiments, 
  onExperimentClick, 
  getRoomColor 
}) => {
  if (isEmpty) {
    return <div className={styles.emptyCell} />;
  }

  const visibleExperiments = experiments.slice(0, MAX_VISIBLE_EXPERIMENTS);
  const hiddenCount = experiments.length - MAX_VISIBLE_EXPERIMENTS;

  return (
    <div className={`${styles.dayCell} ${isToday ? styles.today : ''}`}>
      <div className={`${styles.dayNumber} ${isToday ? styles.todayNumber : ''}`}>
        {day}
      </div>
      <div className={styles.experimentsContainer}>
        {visibleExperiments.map(exp => (
          <ExperimentItem
            key={exp.id}
            experiment={exp}
            onClick={onExperimentClick}
            getRoomColor={getRoomColor}
          />
        ))}
        {hiddenCount > 0 && (
          <div className={styles.moreCount}>
            +{hiddenCount} more
          </div>
        )}
      </div>
    </div>
  );
};

// === Основной компонент ===

const CalendarView = ({ 
  experiments = [], 
  currentDate, 
  setCurrentDate, 
  onExperimentClick,
  getRoomColor 
}) => {
  const year = currentDate.getFullYear();
  const month = currentDate.getMonth();

  // Навигация
  const previousMonth = useCallback(() => {
    setCurrentDate(new Date(year, month - 1, 1));
  }, [year, month, setCurrentDate]);

  const nextMonth = useCallback(() => {
    setCurrentDate(new Date(year, month + 1, 1));
  }, [year, month, setCurrentDate]);

  const goToToday = useCallback(() => {
    setCurrentDate(new Date());
  }, [setCurrentDate]);

  // Вычисляем дни календаря
  const days = useMemo(() => {
    const firstDay = new Date(year, month, 1).getDay();
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    
    const result = [];
    
    // Пустые ячейки до первого дня
    for (let i = 0; i < firstDay; i++) {
      result.push({ day: null, isEmpty: true });
    }
    
    // Дни месяца
    for (let i = 1; i <= daysInMonth; i++) {
      result.push({ day: i, isEmpty: false });
    }
    
    return result;
  }, [year, month]);

  // Группируем эксперименты по дате
  const experimentsByDate = useMemo(() => {
    const map = new Map();
    
    experiments.forEach(exp => {
      const expDate = formatLocalDate(new Date(exp.experiment_date));
      if (!map.has(expDate)) {
        map.set(expDate, []);
      }
      map.get(expDate).push(exp);
    });
    
    return map;
  }, [experiments]);

  // Получить эксперименты для конкретного дня
  const getExperimentsForDay = useCallback((day) => {
    if (!day) return [];
    const dateStr = formatLocalDate(new Date(year, month, day));
    return experimentsByDate.get(dateStr) || [];
  }, [year, month, experimentsByDate]);

  // Проверка на сегодня
  const checkIsToday = useCallback((day) => {
    if (!day) return false;
    const today = new Date();
    return (
      today.getDate() === day && 
      today.getMonth() === month && 
      today.getFullYear() === year
    );
  }, [year, month]);

  return (
    <div className={styles.container}>
      <CalendarHeader
        month={month}
        year={year}
        onPrevious={previousMonth}
        onNext={nextMonth}
        onToday={goToToday}
      />

      <DayNamesRow />

      <div className={styles.grid}>
        {days.map((item, index) => (
          <DayCell
            key={index}
            day={item.day}
            isEmpty={item.isEmpty}
            isToday={checkIsToday(item.day)}
            experiments={getExperimentsForDay(item.day)}
            onExperimentClick={onExperimentClick}
            getRoomColor={getRoomColor}
          />
        ))}
      </div>
    </div>
  );
};

export default CalendarView;
