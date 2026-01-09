// components/FormGroup.js - CHELATE LIMS Style
import React from 'react';

const FormGroup = ({ 
  label, 
  required = false,
  error,
  helpText,
  children,
  style = {},
  className = ''
}) => {
  return (
    <div style={{ marginBottom: '1rem', ...style }} className={className}>
      {label && (
        <label style={{
          display: 'block',
          marginBottom: '6px',
          fontWeight: '600',
          fontSize: '0.8rem',
          color: '#1a365d'
        }}>
          {label}
          {required && (
            <span style={{ color: '#e53e3e', marginLeft: '4px' }}>*</span>
          )}
        </label>
      )}
      
      {children}
      
      {error && (
        <p style={{
          margin: '6px 0 0 0',
          fontSize: '0.75rem',
          color: '#e53e3e',
          display: 'flex',
          alignItems: 'center',
          gap: '4px'
        }}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          {error}
        </p>
      )}
      
      {helpText && !error && (
        <p style={{
          margin: '6px 0 0 0',
          fontSize: '0.75rem',
          color: '#718096'
        }}>
          {helpText}
        </p>
      )}
    </div>
  );
};

export default FormGroup;
