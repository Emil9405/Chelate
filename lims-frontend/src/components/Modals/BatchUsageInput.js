// components/modals/BatchUsageInput.js
// Inline usage input with quantity field or stepper for unit-based dispensing

import React, { useState, useEffect } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import { FlaskIcon } from '../Icons';

// Stepper component for unit-based dispensing
const Stepper = ({ value, onChange, min = 1, max = 999, disabled = false }) => {
  const handleDecrement = () => {
    if (value > min) onChange(value - 1);
  };
  
  const handleIncrement = () => {
    if (value < max) onChange(value + 1);
  };

  return (
    <div style={{ 
      display: 'flex', 
      alignItems: 'center',
      border: '1px solid #e2e8f0',
      borderRadius: '6px',
      overflow: 'hidden',
      height: '32px'
    }}>
      <button
        type="button"
        onClick={handleDecrement}
        disabled={disabled || value <= min}
        style={{
          width: '28px',
          height: '100%',
          border: 'none',
          background: disabled || value <= min ? '#f7fafc' : '#edf2f7',
          cursor: disabled || value <= min ? 'not-allowed' : 'pointer',
          fontSize: '16px',
          fontWeight: 'bold',
          color: disabled || value <= min ? '#a0aec0' : '#4a5568',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }}
      >
        −
      </button>
      <div style={{
        width: '40px',
        textAlign: 'center',
        fontWeight: '600',
        fontSize: '14px',
        color: '#2d3748',
        background: 'white'
      }}>
        {value}
      </div>
      <button
        type="button"
        onClick={handleIncrement}
        disabled={disabled || value >= max}
        style={{
          width: '28px',
          height: '100%',
          border: 'none',
          background: disabled || value >= max ? '#f7fafc' : '#edf2f7',
          cursor: disabled || value >= max ? 'not-allowed' : 'pointer',
          fontSize: '16px',
          fontWeight: 'bold',
          color: disabled || value >= max ? '#a0aec0' : '#4a5568',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }}
      >
        +
      </button>
    </div>
  );
};

// History icon
const ClockIcon = ({ size = 16, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2">
    <circle cx="12" cy="12" r="10"/>
    <polyline points="12 6 12 12 16 14"/>
  </svg>
);

/**
 * BatchUsageInput - инлайн компонент для списания из батча
 * 
 * Props:
 * - batch: объект батча
 * - reagentId: ID реагента
 * - onUsageComplete: callback после успешного списания
 * - onShowHistory: callback для показа истории
 */
export const BatchUsageInput = ({ batch, reagentId, onUsageComplete, onShowHistory }) => {
  const [unitsInfo, setUnitsInfo] = useState(null);
  const [quantity, setQuantity] = useState('');
  const [units, setUnits] = useState(1);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [selectedPlacementId, setSelectedPlacementId] = useState(null);

  const placements = batch?.placements || [];
  const hasMultiplePlacements = placements.length > 1;
  const hasSinglePlacement = placements.length === 1;

  // Auto-select single placement
  useEffect(() => {
    if (hasSinglePlacement) {
      setSelectedPlacementId(placements[0].id);
    } else if (!hasMultiplePlacements) {
      setSelectedPlacementId(null);
    }
  }, [placements.length, hasSinglePlacement, hasMultiplePlacements]);

  const canUseUnits = unitsInfo?.can_dispense_by_units && unitsInfo?.pack_size > 0;
  const availableQuantity = unitsInfo?.available_quantity ?? batch?.quantity ?? 0;
  const availableUnits = unitsInfo?.available_units ?? 0;

  useEffect(() => {
    loadUnitsInfo();
    // eslint-disable-next-line
  }, [batch?.id]);

  const loadUnitsInfo = async () => {
    if (!batch?.id || !reagentId) return;
    try {
      const info = await api.getBatchUnitsInfo(reagentId, batch.id);
      setUnitsInfo(info);
    } catch (err) {
      // Не критично - просто используем данные из batch
      setUnitsInfo(null);
    }
  };

  const handleQuantityUse = async () => {
    const qty = parseFloat(quantity);
    if (!qty || qty <= 0) {
      setError('Enter quantity');
      return;
    }
    if (qty > availableQuantity) {
      setError(`Max: ${availableQuantity}`);
      return;
    }

    setLoading(true);
    setError('');
    setSuccess('');

    try {
      await api.useReagent(reagentId, batch.id, { 
        quantity_used: qty,
        ...(selectedPlacementId ? { placement_id: selectedPlacementId } : {})
      });
      setSuccess(`−${qty} ${batch.unit}`);
      setQuantity('');
      if (onUsageComplete) onUsageComplete();
      // Обновляем info
      loadUnitsInfo();
      // Скрываем успех через 2сек
      setTimeout(() => setSuccess(''), 2000);
    } catch (err) {
      setError(err.message || 'Error');
    } finally {
      setLoading(false);
    }
  };

  const handleUnitsUse = async () => {
    if (units < 1 || units > availableUnits) {
      setError(`Max: ${availableUnits} units`);
      return;
    }

    setLoading(true);
    setError('');
    setSuccess('');

    try {
      const result = await api.dispenseUnits(reagentId, batch.id, { 
        units_to_dispense: units,
        ...(selectedPlacementId ? { placement_id: selectedPlacementId } : {})
      });
      setSuccess(`−${result.units_dispensed} pcs`);
      setUnits(1);
      if (onUsageComplete) onUsageComplete();
      loadUnitsInfo();
      setTimeout(() => setSuccess(''), 2000);
    } catch (err) {
      setError(err.message || 'Error');
    } finally {
      setLoading(false);
    }
  };

  // Room selector for multi-placement batches
  const RoomSelector = () => {
    if (placements.length === 0) return null;

    if (hasSinglePlacement) {
      const p = placements[0];
      return (
        <span style={{
          fontSize: '10px', color: '#4a5568', background: '#edf2f7',
          padding: '2px 6px', borderRadius: '4px', whiteSpace: 'nowrap',
          display: 'inline-flex', alignItems: 'center', gap: '3px',
        }}>
          <span style={{ width: 6, height: 6, borderRadius: '50%', background: p.room_color || '#a0aec0' }} />
          {p.room_name}{p.shelf ? ` / ${p.shelf}` : ''}
        </span>
      );
    }

    return (
      <select
        value={selectedPlacementId || ''}
        onChange={(e) => setSelectedPlacementId(e.target.value || null)}
        style={{
          height: '28px', fontSize: '11px', padding: '0 4px',
          border: !selectedPlacementId ? '1px solid #e53e3e' : '1px solid #e2e8f0',
          borderRadius: '5px', background: 'white', maxWidth: '100px',
          color: '#2d3748',
        }}
      >
        <option value="">Room…</option>
        {placements.map(p => (
          <option key={p.id} value={p.id}>
            {p.room_name}{p.shelf ? ` / ${p.shelf}` : ''} ({p.quantity})
          </option>
        ))}
      </select>
    );
  };

  // Если штучный режим доступен
  if (canUseUnits) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' }}>
        {/* Room selector */}
        <RoomSelector />

        {/* Stepper */}
        <Stepper 
          value={units} 
          onChange={setUnits} 
          min={1} 
          max={availableUnits}
          disabled={loading || availableUnits === 0}
        />
        
        {/* Info tooltip */}
        <span style={{ 
          fontSize: '11px', 
          color: '#718096',
          whiteSpace: 'nowrap'
        }}>
          ×{unitsInfo.pack_size}{batch.unit}
        </span>

        {/* Use button */}
        <Button 
          size="small" 
          variant="secondary" 
          onClick={handleUnitsUse}
          loading={loading}
          disabled={availableUnits === 0 || (hasMultiplePlacements && !selectedPlacementId)}
          icon={<FlaskIcon size={12} />}
          title={`Dispense ${units} × ${unitsInfo.pack_size} = ${units * unitsInfo.pack_size} ${batch.unit}`}
        >
          Use
        </Button>

        {/* History button */}
        <Button 
          size="small" 
          variant="ghost" 
          onClick={() => onShowHistory && onShowHistory(batch)}
          icon={<ClockIcon size={12} />}
          title="View history"
        />

        {/* Success/Error indicators */}
        {success && (
          <span style={{ color: '#38a169', fontSize: '12px', fontWeight: '600' }}>
            ✓ {success}
          </span>
        )}
        {error && (
          <span style={{ color: '#e53e3e', fontSize: '11px' }} title={error}>
            ⚠
          </span>
        )}
      </div>
    );
  }

  // Обычный режим - ввод количества
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' }}>
      {/* Room selector */}
      <RoomSelector />

      {/* Quantity input */}
      <input
        type="number"
        step="0.01"
        min="0.01"
        max={availableQuantity}
        value={quantity}
        onChange={(e) => {
          setQuantity(e.target.value);
          setError('');
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            handleQuantityUse();
          }
        }}
        placeholder={availableQuantity.toString()}
        disabled={loading || availableQuantity === 0}
        style={{
          width: '70px',
          height: '32px',
          padding: '0 8px',
          border: error ? '1px solid #e53e3e' : '1px solid #e2e8f0',
          borderRadius: '6px',
          fontSize: '13px',
          textAlign: 'right'
        }}
      />
      
      {/* Unit label */}
      <span style={{ 
        fontSize: '12px', 
        color: '#718096',
        minWidth: '20px'
      }}>
        {batch.unit}
      </span>

      {/* Use button */}
      <Button 
        size="small" 
        variant="secondary" 
        onClick={handleQuantityUse}
        loading={loading}
        disabled={availableQuantity === 0 || !quantity || (hasMultiplePlacements && !selectedPlacementId)}
        icon={<FlaskIcon size={12} />}
      >
        Use
      </Button>

      {/* History button */}
      <Button 
        size="small" 
        variant="ghost" 
        onClick={() => onShowHistory && onShowHistory(batch)}
        icon={<ClockIcon size={12} />}
        title="View history"
      />

      {/* Success/Error indicators */}
      {success && (
        <span style={{ color: '#38a169', fontSize: '12px', fontWeight: '600' }}>
          ✓ {success}
        </span>
      )}
      {error && (
        <span style={{ color: '#e53e3e', fontSize: '11px' }} title={error}>
          ⚠
        </span>
      )}
    </div>
  );
};

export default BatchUsageInput;
