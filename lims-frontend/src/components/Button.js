// components/Button.js - CHELATE LIMS Style
import React from 'react';
import { Spinner } from './Icons';

const Button = ({ 
  onClick, 
  children, 
  variant = 'primary', 
  size = 'medium',
  disabled = false, 
  loading = false,
  icon,
  iconPosition = 'left',
  type = 'button',
  fullWidth = false,
  style = {},
  ...props 
}) => {
  const sizeStyles = {
    small: {
      padding: '6px 12px',
      fontSize: '0.75rem',
      borderRadius: '6px',
      gap: '4px'
    },
    medium: {
      padding: '10px 18px',
      fontSize: '0.875rem',
      borderRadius: '10px',
      gap: '8px'
    },
    large: {
      padding: '14px 24px',
      fontSize: '1rem',
      borderRadius: '12px',
      gap: '10px'
    }
  };

  const variantStyles = {
    primary: { 
      background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
      color: 'white',
      border: 'none',
      boxShadow: '0 4px 15px rgba(49, 130, 206, 0.3)'
    },
    secondary: { 
      background: 'white',
      color: '#1a365d',
      border: '2px solid #e2e8f0',
      boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)'
    },
    success: { 
      background: 'linear-gradient(135deg, #38a169 0%, #2f855a 100%)',
      color: 'white',
      border: 'none',
      boxShadow: '0 4px 15px rgba(56, 161, 105, 0.3)'
    },
    danger: { 
      background: 'linear-gradient(135deg, #e53e3e 0%, #c53030 100%)',
      color: 'white',
      border: 'none',
      boxShadow: '0 4px 15px rgba(229, 62, 62, 0.3)'
    },
    warning: {
      background: 'linear-gradient(135deg, #ed8936 0%, #dd6b20 100%)',
      color: 'white',
      border: 'none',
      boxShadow: '0 4px 15px rgba(237, 137, 54, 0.3)'
    },
    ghost: {
      background: 'transparent',
      color: '#3182ce',
      border: 'none',
      boxShadow: 'none'
    },
    outline: {
      background: 'transparent',
      color: '#3182ce',
      border: '2px solid #3182ce',
      boxShadow: 'none'
    }
  };

  const baseStyle = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontWeight: '600',
    cursor: (disabled || loading) ? 'not-allowed' : 'pointer',
    transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
    opacity: (disabled || loading) ? 0.6 : 1,
    fontFamily: 'inherit',
    position: 'relative',
    overflow: 'hidden',
    width: fullWidth ? '100%' : 'auto',
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...style
  };

  // Render icon with proper sizing
  const renderIcon = (iconElement) => {
    if (!iconElement) return null;
    
    if (React.isValidElement(iconElement)) {
      const iconSize = size === 'small' ? 14 : size === 'large' ? 20 : 16;
      return React.cloneElement(iconElement, { 
        size: iconElement.props.size || iconSize,
        color: iconElement.props.color || 'currentColor'
      });
    }
    
    return iconElement;
  };

  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled || loading}
      style={baseStyle}
      {...props}
    >
      {loading ? (
        <>
          <Spinner size={size === 'small' ? 14 : size === 'large' ? 20 : 16} color="currentColor" />
          <span>Loading...</span>
        </>
      ) : (
        <>
          {icon && iconPosition === 'left' && renderIcon(icon)}
          {children}
          {icon && iconPosition === 'right' && renderIcon(icon)}
        </>
      )}
    </button>
  );
};

export default Button;
