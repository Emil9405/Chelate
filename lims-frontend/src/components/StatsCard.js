// components/StatsCard.js - CHELATE LIMS Style with SVG Support
import React from 'react';
import { TrendUpIcon, TrendDownIcon } from './Icons';

const StatsCard = ({ 
  title, 
  value, 
  icon, 
  variant = 'primary',
  trend,
  trendValue,
  subtitle,
  onClick 
}) => {
  const variants = {
    primary: {
      gradient: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
      iconBg: 'rgba(49, 130, 206, 0.1)',
      iconColor: '#3182ce'
    },
    success: {
      gradient: 'linear-gradient(135deg, #38a169 0%, #2f855a 100%)',
      iconBg: 'rgba(56, 161, 105, 0.1)',
      iconColor: '#38a169'
    },
    warning: {
      gradient: 'linear-gradient(135deg, #ed8936 0%, #dd6b20 100%)',
      iconBg: 'rgba(237, 137, 54, 0.1)',
      iconColor: '#ed8936'
    },
    danger: {
      gradient: 'linear-gradient(135deg, #e53e3e 0%, #c53030 100%)',
      iconBg: 'rgba(229, 62, 62, 0.1)',
      iconColor: '#e53e3e'
    },
    teal: {
      gradient: 'linear-gradient(135deg, #38b2ac 0%, #319795 100%)',
      iconBg: 'rgba(56, 178, 172, 0.1)',
      iconColor: '#38b2ac'
    },
    info: {
      gradient: 'linear-gradient(135deg, #4299e1 0%, #3182ce 100%)',
      iconBg: 'rgba(66, 153, 225, 0.1)',
      iconColor: '#4299e1'
    }
  };

  const currentVariant = variants[variant] || variants.primary;
  
  // Render icon - supports both React components and strings
  const renderIcon = () => {
    if (!icon) return null;
    
    // If icon is already a React element (SVG component)
    if (React.isValidElement(icon)) {
      return React.cloneElement(icon, { 
        color: currentVariant.iconColor,
        size: icon.props.size || 24
      });
    }
    
    // If icon is a string (fallback for old usage)
    if (typeof icon === 'string') {
      return <span style={{ fontSize: '1.5rem' }}>{icon}</span>;
    }
    
    return icon;
  };

  return (
    <div 
      onClick={onClick}
      style={{
        background: 'white',
        borderRadius: '16px',
        padding: '24px',
        position: 'relative',
        overflow: 'hidden',
        cursor: onClick ? 'pointer' : 'default',
        transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
        border: '1px solid #e2e8f0',
        boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)'
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.transform = 'translateY(-4px)';
        e.currentTarget.style.boxShadow = '0 12px 30px rgba(26, 54, 93, 0.12)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.transform = 'translateY(0)';
        e.currentTarget.style.boxShadow = '0 1px 3px rgba(26, 54, 93, 0.08)';
      }}
    >
      {/* Top gradient border */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        height: '4px',
        background: currentVariant.gradient
      }}></div>

      {/* Molecular decoration */}
      <div style={{
        position: 'absolute',
        top: '15px',
        right: '15px',
        width: '60px',
        height: '60px',
        opacity: 0.05,
        transition: 'opacity 0.3s ease'
      }}>
        <svg viewBox="0 0 60 60">
          <circle cx="30" cy="30" r="10" fill="#1a365d"/>
          <circle cx="12" cy="12" r="5" fill="#1a365d"/>
          <circle cx="48" cy="12" r="5" fill="#1a365d"/>
          <circle cx="12" cy="48" r="5" fill="#1a365d"/>
          <circle cx="48" cy="48" r="5" fill="#1a365d"/>
          <line x1="30" y1="30" x2="12" y2="12" stroke="#1a365d" strokeWidth="2"/>
          <line x1="30" y1="30" x2="48" y2="12" stroke="#1a365d" strokeWidth="2"/>
          <line x1="30" y1="30" x2="12" y2="48" stroke="#1a365d" strokeWidth="2"/>
          <line x1="30" y1="30" x2="48" y2="48" stroke="#1a365d" strokeWidth="2"/>
        </svg>
      </div>

      {/* Header with Icon */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'flex-start',
        marginBottom: '16px'
      }}>
        {/* Icon */}
        <div style={{
          width: '52px',
          height: '52px',
          borderRadius: '12px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '1.5rem',
          background: currentVariant.iconBg,
          color: currentVariant.iconColor,
          transition: 'transform 0.3s ease'
        }}>
          {renderIcon()}
        </div>

        {/* Trend Badge */}
        {trend && (
          <div style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '4px 10px',
            borderRadius: '20px',
            fontSize: '0.75rem',
            fontWeight: '600',
            background: trend === 'up' 
              ? 'rgba(56, 161, 105, 0.1)' 
              : 'rgba(229, 62, 62, 0.1)',
            color: trend === 'up' ? '#2f855a' : '#c53030'
          }}>
            {trend === 'up' ? (
              <TrendUpIcon size={12} color="#2f855a" />
            ) : (
              <TrendDownIcon size={12} color="#c53030" />
            )}
            {trendValue}
          </div>
        )}
      </div>

      {/* Value */}
      <div style={{
        fontSize: '2.25rem',
        fontWeight: '800',
        color: '#1a365d',
        lineHeight: 1,
        marginBottom: '6px',
        fontFeatureSettings: '"tnum"'
      }}>
        {value}
      </div>

      {/* Title */}
      <div style={{
        fontSize: '0.875rem',
        color: '#718096',
        fontWeight: '500'
      }}>
        {title}
      </div>

      {/* Subtitle/Additional Info */}
      {subtitle && (
        <div style={{
          fontSize: '0.75rem',
          color: currentVariant.iconColor,
          marginTop: '8px',
          fontWeight: '500'
        }}>
          {subtitle}
        </div>
      )}
    </div>
  );
};

export default StatsCard;
