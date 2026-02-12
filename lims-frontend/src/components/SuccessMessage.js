// components/SuccessMessage.js
import React from 'react';

const SuccessMessage = ({ message, onDismiss }) => (
  <div style={{
    backgroundColor: '#c6f6d5',
    color: '#22543d',
    padding: '12px 16px',
    borderRadius: '8px',
    marginBottom: '1rem',
    fontSize: '14px',
    position: 'relative'
  }}>
    <span>{message}</span>
    {onDismiss && (
      <button 
        onClick={onDismiss}
        style={{ 
          position: 'absolute',
          right: '16px',
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          fontSize: '18px',
          color: '#22543d'
        }}
      >
        ×
      </button>
    )}
  </div>
);

export default SuccessMessage;