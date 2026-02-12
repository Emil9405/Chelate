// components/Toast.js - CHELATE LIMS Style
import React, { useEffect, useState } from 'react';
import { CheckCircleIcon, AlertCircleIcon, AlertTriangleIcon, InfoIcon, CloseIcon } from './Icons';
import './Toast.css';

const Toast = ({ message, type = 'info', duration = 5000, onClose, title }) => {
  const [isExiting, setIsExiting] = useState(false);

  useEffect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        handleClose();
      }, duration);
      return () => clearTimeout(timer);
    }
  }, [duration]);

  const handleClose = () => {
    setIsExiting(true);
    setTimeout(() => {
      if (onClose) onClose();
    }, 300);
  };

  const icons = {
    success: CheckCircleIcon,
    error: AlertCircleIcon,
    warning: AlertTriangleIcon,
    info: InfoIcon
  };

  const colors = {
    success: { bg: 'rgba(56, 161, 105, 0.1)', color: '#38a169' },
    error: { bg: 'rgba(229, 62, 62, 0.1)', color: '#e53e3e' },
    warning: { bg: 'rgba(237, 137, 54, 0.1)', color: '#ed8936' },
    info: { bg: 'rgba(49, 130, 206, 0.1)', color: '#3182ce' }
  };

  const IconComponent = icons[type] || InfoIcon;
  const colorConfig = colors[type] || colors.info;

  const titles = {
    success: 'Success',
    error: 'Error',
    warning: 'Warning',
    info: 'Information'
  };

  return (
    <div className={`toast ${type} ${isExiting ? 'exiting' : ''}`}>
      <div className="toast-icon" style={{ background: colorConfig.bg }}>
        <IconComponent size={20} color={colorConfig.color} />
      </div>
      <div className="toast-content">
        <div className="toast-title">{title || titles[type]}</div>
        <div className="toast-message">{message}</div>
      </div>
      <button className="toast-close" onClick={handleClose}>
        <CloseIcon size={16} />
      </button>
    </div>
  );
};

// Toast Container Component
export const ToastContainer = ({ toasts, removeToast }) => {
  return (
    <div className="toast-container">
      {toasts.map((toast) => (
        <Toast
          key={toast.id}
          {...toast}
          onClose={() => removeToast(toast.id)}
        />
      ))}
    </div>
  );
};

// Custom hook for toast management
export const useToast = () => {
  const [toasts, setToasts] = useState([]);

  const addToast = (message, type = 'info', options = {}) => {
    const id = Date.now() + Math.random();
    const newToast = {
      id,
      message,
      type,
      ...options
    };
    setToasts((prev) => [...prev, newToast]);
    return id;
  };

  const removeToast = (id) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id));
  };

  const showSuccess = (message, options) => addToast(message, 'success', options);
  const showError = (message, options) => addToast(message, 'error', options);
  const showWarning = (message, options) => addToast(message, 'warning', options);
  const showInfo = (message, options) => addToast(message, 'info', options);

  return {
    toasts,
    addToast,
    removeToast,
    showSuccess,
    showError,
    showWarning,
    showInfo
  };
};

export default Toast;
