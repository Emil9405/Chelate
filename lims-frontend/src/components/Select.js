// components/Select.js - CHELATE LIMS Style
import React from 'react';
import { ChevronDownIcon } from './Icons';

const Select = React.forwardRef(({ 
  error,
  success,
  size = 'medium',
  fullWidth = true,
  style = {},
  className = '',
  children,
  ...props 
}, ref) => {
  const sizeStyles = {
    small: {
      padding: '8px 32px 8px 12px',
      fontSize: '0.8rem',
      borderRadius: '8px'
    },
    medium: {
      padding: '12px 40px 12px 16px',
      fontSize: '0.875rem',
      borderRadius: '10px'
    },
    large: {
      padding: '14px 44px 14px 18px',
      fontSize: '1rem',
      borderRadius: '12px'
    }
  };

  const getBorderColor = () => {
    if (error) return '#e53e3e';
    if (success) return '#38a169';
    return '#e2e8f0';
  };

  const baseStyle = {
    width: fullWidth ? '100%' : 'auto',
    border: `2px solid ${getBorderColor()}`,
    backgroundColor: '#fff',
    color: '#1a365d',
    fontFamily: 'inherit',
    transition: 'all 0.2s ease',
    outline: 'none',
    boxSizing: 'border-box',
    appearance: 'none',
    cursor: 'pointer',
    backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%23718096' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E")`,
    backgroundRepeat: 'no-repeat',
    backgroundPosition: 'right 12px center',
    ...sizeStyles[size],
    ...style
  };

  const handleFocus = (e) => {
    e.target.style.borderColor = error ? '#e53e3e' : success ? '#38a169' : '#3182ce';
    e.target.style.boxShadow = `0 0 0 4px ${error ? 'rgba(229, 62, 62, 0.15)' : success ? 'rgba(56, 161, 105, 0.15)' : 'rgba(49, 130, 206, 0.15)'}`;
    props.onFocus?.(e);
  };

  const handleBlur = (e) => {
    e.target.style.borderColor = getBorderColor();
    e.target.style.boxShadow = 'none';
    props.onBlur?.(e);
  };

  return (
    <select
      ref={ref}
      className={className}
      style={baseStyle}
      onFocus={handleFocus}
      onBlur={handleBlur}
      {...props}
    >
      {children}
    </select>
  );
});

Select.displayName = 'Select';

export default Select;
