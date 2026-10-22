// components/labels/LabelIcons.js
import React from 'react';

/** Stylised Data Matrix: L-shaped finder + a few modules */
export const DataMatrixIcon = ({ size = 18, color = 'currentColor', style }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill={color} aria-hidden="true"
    style={{ display: 'inline-block', verticalAlign: 'middle', ...style }}>
    <path d="M3 3h2v16h16v2H3z" />
    <path d="M7 3h2v2H7zM11 3h2v2h-2zM15 3h2v2h-2zM19 5h2v2h-2zM19 9h2v2h-2zM19 13h2v2h-2zM19 17h2v2h-2z" />
    <path d="M9 7h3v3H9zM14 8h2v2h-2zM8 12h2v3H8zM12 12h3v2h-3zM16 12h2v2h-2zM11 15h2v2h-2zM15 15h2v2h-2z" />
  </svg>
);

/** Camera with a scan frame */
export const ScanIcon = ({ size = 20, color = 'currentColor', style }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"
    style={{ display: 'inline-block', verticalAlign: 'middle', ...style }}>
    <path d="M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2" />
    <rect x="7" y="7" width="10" height="10" rx="1" />
  </svg>
);
