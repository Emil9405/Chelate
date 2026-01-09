// components/TextArea.js - CHELATE LIMS Style
import React from 'react';

const TextArea = React.forwardRef(({ 
  error,
  success,
  size = 'medium',
  fullWidth = true,
  rows = 4,
  resize = 'vertical',
  style = {},
  className = '',
  ...props 
}, ref) => {
  const sizeStyles = {
    small: {
      padding: '8px 12px',
      fontSize: '0.8rem',
      borderRadius: '8px'
    },
    medium: {
      padding: '12px 16px',
      fontSize: '0.875rem',
      borderRadius: '10px'
    },
    large: {
      padding: '14px 18px',
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
    resize: resize,
    lineHeight: '1.5',
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
    <textarea
      ref={ref}
      rows={rows}
      className={className}
      style={baseStyle}
      onFocus={handleFocus}
      onBlur={handleBlur}
      {...props}
    />
  );
});

TextArea.displayName = 'TextArea';

export default TextArea;
