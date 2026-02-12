// components/Badge.js - CHELATE LIMS Style
import React from 'react';

const Badge = ({ children, variant = 'default', size = 'medium', dot = false, style = {} }) => {
  const sizeStyles = {
    small: {
      padding: dot ? '0' : '2px 8px',
      fontSize: '0.65rem',
      width: dot ? '8px' : 'auto',
      height: dot ? '8px' : 'auto'
    },
    medium: {
      padding: dot ? '0' : '4px 10px',
      fontSize: '0.75rem',
      width: dot ? '10px' : 'auto',
      height: dot ? '10px' : 'auto'
    },
    large: {
      padding: dot ? '0' : '6px 14px',
      fontSize: '0.8rem',
      width: dot ? '12px' : 'auto',
      height: dot ? '12px' : 'auto'
    }
  };

  const variantStyles = {
    default: {
      background: 'rgba(113, 128, 150, 0.1)',
      color: '#718096'
    },
    primary: {
      background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.15) 0%, rgba(56, 178, 172, 0.15) 100%)',
      color: '#2b6cb0'
    },
    success: {
      background: 'linear-gradient(135deg, rgba(56, 161, 105, 0.15) 0%, rgba(47, 133, 90, 0.15) 100%)',
      color: '#2f855a'
    },
    warning: {
      background: 'linear-gradient(135deg, rgba(237, 137, 54, 0.15) 0%, rgba(221, 107, 32, 0.15) 100%)',
      color: '#c05621'
    },
    danger: {
      background: 'linear-gradient(135deg, rgba(229, 62, 62, 0.15) 0%, rgba(197, 48, 48, 0.15) 100%)',
      color: '#c53030'
    },
    info: {
      background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.15) 0%, rgba(66, 153, 225, 0.15) 100%)',
      color: '#2b6cb0'
    },
    teal: {
      background: 'rgba(56, 178, 172, 0.15)',
      color: '#319795'
    },
    // Solid variants
    'primary-solid': {
      background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
      color: 'white'
    },
    'success-solid': {
      background: 'linear-gradient(135deg, #38a169 0%, #2f855a 100%)',
      color: 'white'
    },
    'warning-solid': {
      background: 'linear-gradient(135deg, #ed8936 0%, #dd6b20 100%)',
      color: 'white'
    },
    'danger-solid': {
      background: 'linear-gradient(135deg, #e53e3e 0%, #c53030 100%)',
      color: 'white'
    }
  };

  const baseStyle = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: dot ? '50%' : '20px',
    fontWeight: '700',
    textTransform: 'uppercase',
    letterSpacing: '0.03em',
    lineHeight: 1,
    whiteSpace: 'nowrap',
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...style
  };

  if (dot) {
    return <span style={baseStyle} />;
  }

  return (
    <span style={baseStyle}>
      {children}
    </span>
  );
};

export default Badge;
