// components/Loading.js - CHELATE LIMS Style
import React from 'react';
import { ChelateLogo, Spinner } from './Icons';

// Simple Spinner Component
export const Loading = ({ size = 'medium', color = '#3182ce', text }) => {
  const sizes = {
    small: 20,
    medium: 32,
    large: 48
  };

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: '12px'
    }}>
      <Spinner size={sizes[size] || size} color={color} />
      {text && (
        <span style={{
          color: '#718096',
          fontSize: '0.875rem',
          fontWeight: '500'
        }}>
          {text}
        </span>
      )}
    </div>
  );
};

// Full Page Loading Overlay
export const LoadingOverlay = ({ message = 'Loading...', showLogo = true }) => {
  return (
    <div style={{
      position: 'fixed',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      background: 'rgba(255, 255, 255, 0.95)',
      backdropFilter: 'blur(8px)',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 9999,
      gap: '24px'
    }}>
      {showLogo && (
        <div style={{ marginBottom: '8px' }}>
          <ChelateLogo size={60} />
        </div>
      )}
      
      {/* Animated loader */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: '8px'
      }}>
        <div style={{
          width: '12px',
          height: '12px',
          borderRadius: '50%',
          background: '#3182ce',
          animation: 'bounce 1.4s ease-in-out infinite both',
          animationDelay: '-0.32s'
        }} />
        <div style={{
          width: '12px',
          height: '12px',
          borderRadius: '50%',
          background: '#38b2ac',
          animation: 'bounce 1.4s ease-in-out infinite both',
          animationDelay: '-0.16s'
        }} />
        <div style={{
          width: '12px',
          height: '12px',
          borderRadius: '50%',
          background: '#38a169',
          animation: 'bounce 1.4s ease-in-out infinite both'
        }} />
      </div>
      
      <p style={{
        color: '#1a365d',
        fontSize: '1rem',
        fontWeight: '500',
        margin: 0
      }}>
        {message}
      </p>

      <style>{`
        @keyframes bounce {
          0%, 80%, 100% {
            transform: scale(0);
            opacity: 0.5;
          }
          40% {
            transform: scale(1);
            opacity: 1;
          }
        }
        
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
};

// Inline/Section Loading
export const SectionLoading = ({ height = '200px', message }) => {
  return (
    <div style={{
      height,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'linear-gradient(180deg, rgba(49, 130, 206, 0.03) 0%, rgba(56, 161, 105, 0.03) 100%)',
      borderRadius: '12px',
      gap: '12px'
    }}>
      <Spinner size={32} color="#3182ce" />
      {message && (
        <span style={{
          color: '#718096',
          fontSize: '0.875rem'
        }}>
          {message}
        </span>
      )}
    </div>
  );
};

// Skeleton Loading
export const Skeleton = ({ width = '100%', height = '20px', borderRadius = '6px', style = {} }) => {
  return (
    <div
      style={{
        width,
        height,
        borderRadius,
        background: 'linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%)',
        backgroundSize: '200% 100%',
        animation: 'shimmer 1.5s infinite',
        ...style
      }}
    >
      <style>{`
        @keyframes shimmer {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }
      `}</style>
    </div>
  );
};

export default Loading;
