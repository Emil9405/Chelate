// components/Modal.js - CHELATE LIMS Style
import React, { useEffect } from 'react';
import { CloseIcon } from './Icons';

const Modal = ({ 
  isOpen, 
  onClose, 
  title, 
  children, 
  size = 'medium',
  showClose = true,
  footer,
  className = ''
}) => {
  // Handle ESC key press
  useEffect(() => {
    const handleEsc = (e) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    
    if (isOpen) {
      document.addEventListener('keydown', handleEsc);
      document.body.style.overflow = 'hidden';
    }
    
    return () => {
      document.removeEventListener('keydown', handleEsc);
      document.body.style.overflow = 'unset';
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const sizeStyles = {
    small: { maxWidth: '400px' },
    medium: { maxWidth: '600px' },
    large: { maxWidth: '800px' },
    xlarge: { maxWidth: '1000px' },
    full: { maxWidth: '95vw', width: '95vw' }
  };

  const handleOverlayClick = (e) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  return (
    <div 
      className={`modal-overlay ${className}`}
      onClick={handleOverlayClick}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(26, 54, 93, 0.6)',
        backdropFilter: 'blur(6px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 2000,
        padding: '20px',
        animation: 'fadeIn 0.2s ease'
      }}
    >
      <div 
        className="modal-content"
        style={{
          background: 'white',
          borderRadius: '20px',
          width: '100%',
          maxHeight: '90vh',
          overflow: 'hidden',
          boxShadow: '0 25px 60px rgba(26, 54, 93, 0.25)',
          animation: 'slideUp 0.3s ease',
          position: 'relative',
          display: 'flex',
          flexDirection: 'column',
          ...sizeStyles[size]
        }}
      >
        {/* Gradient top border */}
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          height: '4px',
          background: 'linear-gradient(90deg, #3182ce 0%, #38b2ac 50%, #38a169 100%)',
          borderRadius: '20px 20px 0 0'
        }} />

        {/* Header */}
        {(title || showClose) && (
          <div style={{
            padding: '20px 24px',
            borderBottom: '1px solid #e2e8f0',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            flexShrink: 0
          }}>
            <h2 style={{
              fontSize: '1.25rem',
              fontWeight: '700',
              color: '#1a365d',
              margin: 0
            }}>
              {title}
            </h2>
            {showClose && (
              <button
                onClick={onClose}
                style={{
                  width: '36px',
                  height: '36px',
                  borderRadius: '10px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  background: '#f7fafc',
                  border: 'none',
                  color: '#718096',
                  cursor: 'pointer',
                  transition: 'all 0.2s ease'
                }}
              >
                <CloseIcon size={18} />
              </button>
            )}
          </div>
        )}

        {/* Body */}
        <div style={{
          padding: '24px',
          overflowY: 'auto',
          flex: 1
        }}>
          {children}
        </div>

        {/* Footer */}
        {footer && (
          <div style={{
            padding: '16px 24px',
            borderTop: '1px solid #e2e8f0',
            display: 'flex',
            justifyContent: 'flex-end',
            gap: '12px',
            background: '#f8fafc',
            flexShrink: 0
          }}>
            {footer}
          </div>
        )}
      </div>

      {/* CSS for animations */}
      <style>{`
        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        
        @keyframes slideUp {
          from {
            opacity: 0;
            transform: translateY(20px) scale(0.98);
          }
          to {
            opacity: 1;
            transform: translateY(0) scale(1);
          }
        }
      `}</style>
    </div>
  );
};

export default Modal;
