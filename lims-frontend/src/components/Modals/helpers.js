// components/modals/helpers.js - Shared utilities and hooks

import { useCallback } from 'react';

/**
 * Custom hook for form submission with validation
 */
export const useFormSubmit = (onSubmit, validate) => {
  const handleSubmit = useCallback(async (e) => {
    e.preventDefault();
    if (validate && !validate()) return;
    try {
      await onSubmit();
    } catch (err) {
      console.error('Form submit error:', err);
    }
  }, [onSubmit, validate]);
  return handleSubmit;
};

/**
 * Clean payload by removing empty strings and null values
 */
export function cleanPayload(data) {
  const payload = { ...data };
  Object.keys(payload).forEach(key => {
    if (payload[key] === '' || payload[key] === null) {
      delete payload[key];
    }
  });
  return payload;
}

/**
 * Format date for display
 */
export function formatDate(dateString, locale = 'ru-RU') {
  if (!dateString) return '—';
  return new Date(dateString).toLocaleDateString(locale);
}

/**
 * Get expiry status with color and text
 */
export function getExpiryStatus(expiryDate) {
  if (!expiryDate) return { color: '#718096', text: 'N/A' };
  
  const expiry = new Date(expiryDate);
  const now = new Date();
  const days = Math.ceil((expiry - now) / (1000 * 60 * 60 * 24));
  
  if (days < 0) {
    return { color: '#e53e3e', text: `Expired` };
  }
  if (days <= 30) {
    return { color: '#ed8936', text: `${days}d left` };
  }
  return { color: '#38a169', text: expiry.toLocaleDateString('ru-RU') };
}

export default {
  useFormSubmit,
  cleanPayload,
  formatDate,
  getExpiryStatus
};
