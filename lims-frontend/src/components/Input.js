// components/Input.js - CHELATE LIMS Style
import React from 'react';

const Input = React.forwardRef(({ 
  type = 'text',
  error,
  success,
  icon,
  iconPosition = 'left',
  size = 'medium',
  fullWidth = true,
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

  const getFocusColor = () => {
    if (error) return 'rgba(229, 62, 62, 0.15)';
    if (success) return 'rgba(56, 161, 105, 0.15)';
    return 'rgba(49, 130, 206, 0.15)';
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
    ...sizeStyles[size],
    ...(icon && iconPosition === 'left' ? { paddingLeft: '44px' } : {}),
    ...(icon && iconPosition === 'right' ? { paddingRight: '44px' } : {}),
    ...style
  };

  const handleFocus = (e) => {
    e.target.style.borderColor = error ? '#e53e3e' : success ? '#38a169' : '#3182ce';
    e.target.style.boxShadow = `0 0 0 4px ${getFocusColor()}`;
    props.onFocus?.(e);
  };

  const handleBlur = (e) => {
    e.target.style.borderColor = getBorderColor();
    e.target.style.boxShadow = 'none';
    props.onBlur?.(e);
  };

  return (
    <div style={{ position: 'relative', width: fullWidth ? '100%' : 'auto' }}>
      {icon && iconPosition === 'left' && (
        <div style={{
          position: 'absolute',
          left: '14px',
          top: '50%',
          transform: 'translateY(-50%)',
          pointerEvents: 'none',
          color: '#a0aec0',
          display: 'flex',
          alignItems: 'center'
        }}>
          {icon}
        </div>
      )}
      
      <input
        ref={ref}
        type={type}
        className={className}
        style={baseStyle}
        onFocus={handleFocus}
        onBlur={handleBlur}
        {...props}
      />
      
      {icon && iconPosition === 'right' && (
        <div style={{
          position: 'absolute',
          right: '14px',
          top: '50%',
          transform: 'translateY(-50%)',
          pointerEvents: 'none',
          color: '#a0aec0',
          display: 'flex',
          alignItems: 'center'
        }}>
          {icon}
        </div>
      )}
    </div>
  );
});

Input.displayName = 'Input';

export default Input;
