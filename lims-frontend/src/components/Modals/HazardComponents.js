// components/modals/HazardComponents.js - GHS Hazard Pictogram Components

import React from 'react';

// GHS Pictograms Data
export const GHS_PICTOGRAMS = [
  { code: 'GHS01', label: 'Explosive', icon: '💥' },
  { code: 'GHS02', label: 'Flammable', icon: '🔥' },
  { code: 'GHS03', label: 'Oxidizing', icon: '⭕' },
  { code: 'GHS04', label: 'Gas under pressure', icon: '🫧' },
  { code: 'GHS05', label: 'Corrosive', icon: '⚗️' },
  { code: 'GHS06', label: 'Toxic', icon: '☠️' },
  { code: 'GHS07', label: 'Irritant', icon: '⚠️' },
  { code: 'GHS08', label: 'Danger to health', icon: '🫁' },
  { code: 'GHS09', label: 'Danger to environment', icon: '🌊' },
];

/**
 * GHS Hazard Selector Component
 * Allows multi-select of GHS pictograms
 */
export const HazardSelect = ({ selectedCodes, onChange }) => {
  const currentSelection = typeof selectedCodes === 'string' && selectedCodes 
    ? selectedCodes.split(',').filter(Boolean)
    : (Array.isArray(selectedCodes) ? selectedCodes : []);

  const toggleCode = (code) => {
    let newSelection;
    if (currentSelection.includes(code)) {
      newSelection = currentSelection.filter(c => c !== code);
    } else {
      newSelection = [...currentSelection, code];
    }
    onChange(newSelection.join(','));
  };

  return (
    <div>
      <div style={{ 
        display: 'grid', 
        gridTemplateColumns: 'repeat(auto-fill, minmax(75px, 1fr))',
        gap: '8px',
        marginBottom: '8px'
      }}>
        {GHS_PICTOGRAMS.map((item) => {
          const isSelected = currentSelection.includes(item.code);
          return (
            <div
              key={item.code}
              onClick={() => toggleCode(item.code)}
              title={item.label}
              style={{
                cursor: 'pointer',
                border: isSelected ? '2px solid #3182ce' : '1px solid #e2e8f0',
                backgroundColor: isSelected ? '#ebf8ff' : '#fff',
                borderRadius: '6px',
                padding: '8px 4px',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'all 0.15s ease',
                minHeight: '75px'
              }}
            >
              <img 
                src={`/assets/ghs/${item.code}.svg`}
                alt={item.code}
                style={{ 
                  width: '40px', 
                  height: '40px', 
                  objectFit: 'contain', 
                  opacity: isSelected ? 1 : 0.7 
                }}
                onError={(e) => {
                  if (!e.target.dataset.tried) {
                    e.target.dataset.tried = 'png';
                    e.target.src = `/assets/ghs/${item.code}.png`;
                  }
                }}
              />
              <span style={{ 
                fontSize: '9px', 
                fontWeight: isSelected ? '600' : '500', 
                marginTop: '4px', 
                color: isSelected ? '#2c5282' : '#718096', 
                textAlign: 'center' 
              }}>
                {item.code}
              </span>
            </div>
          );
        })}
      </div>
      {currentSelection.length > 0 && (
        <div style={{ fontSize: '0.8rem', color: '#4a5568' }}>
          Selected: {currentSelection.join(', ')}
        </div>
      )}
    </div>
  );
};

/**
 * GHS Hazard Display Component
 * Displays GHS pictograms in a diamond pattern
 */
export const HazardDisplay = ({ codes }) => {
  if (!codes) return <span style={{ color: '#a0aec0' }}>—</span>;
  
  const codeList = codes.split(',').filter(Boolean);
  if (codeList.length === 0) return <span style={{ color: '#a0aec0' }}>—</span>;

  const iconSize = 80;
  const gap = 40; 
  const step = iconSize / 32; 
  
  const bottomRow = codeList.filter((_, i) => i % 2 === 0);
  const topRow = codeList.filter((_, i) => i % 2 === 1);

  const renderIcon = (code) => {
    const pic = GHS_PICTOGRAMS.find(p => p.code === code);
    return (
      <img 
        key={code}
        src={`/assets/ghs/${code}.svg`}
        alt={pic?.label || code}
        title={pic?.label || code}
        style={{ width: iconSize, height: iconSize, objectFit: 'contain' }}
        onError={(e) => {
          if (!e.target.dataset.tried) {
            e.target.dataset.tried = 'png';
            e.target.src = `/assets/ghs/${code}.png`;
          }
        }}
      />
    );
  };

  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column' }}>
      {topRow.length > 0 && (
        <div style={{ 
          display: 'flex', 
          gap: step, 
          marginLeft: iconSize / 2 + 1, 
          marginBottom: -gap 
        }}>
          {topRow.map(renderIcon)}
        </div>
      )}
      <div style={{ display: 'flex', gap: step }}>
        {bottomRow.map(renderIcon)}
      </div>
    </div>
  );
};

export default {
  GHS_PICTOGRAMS,
  HazardSelect,
  HazardDisplay
};
