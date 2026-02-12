// components/ErrorMessage.js
import React from 'react';

const ErrorMessage = ({ message, onDismiss }) => (
  <div style={{
    backgroundColor: '#fed7d7',
    color: '#c53030',
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
          color: '#c53030'
        }}
      >
        ×
      </button>
    )}
  </div>
);

export default ErrorMessage;