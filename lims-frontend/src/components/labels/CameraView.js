// components/labels/CameraView.js
// Square camera preview with the scan guide, for useCameraScanner.
// Keep it mounted (pass `hidden`) so the stream can be re-attached.

import React from 'react';
import Button from '../Button';
import { GUIDE } from './useCameraScanner';

const CameraView = ({ scanner, hidden = false, flash = null }) => (
  <div style={{
    position: 'relative', width: '100%', aspectRatio: '1 / 1', background: '#1a202c',
    borderRadius: '16px', overflow: 'hidden', display: hidden ? 'none' : 'block',
  }}>
    <video ref={scanner.videoRef} playsInline muted autoPlay
      style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />
    <div aria-hidden="true" style={{
      position: 'absolute', left: `${(1 - GUIDE) * 50}%`, top: `${(1 - GUIDE) * 50}%`,
      width: `${GUIDE * 100}%`, height: `${GUIDE * 100}%`,
      border: `3px solid ${flash || 'rgba(255,255,255,0.9)'}`, borderRadius: '12px',
      boxShadow: '0 0 0 9999px rgba(0,0,0,0.35)', transition: 'border-color 0.15s',
    }} />
    {!scanner.cameraOn && (
      <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Button variant="primary" onClick={scanner.start}>Start camera</Button>
      </div>
    )}
    {scanner.torch !== null && scanner.cameraOn && (
      <button type="button" onClick={scanner.toggleTorch} aria-pressed={scanner.torch}
        style={{
          position: 'absolute', right: '12px', bottom: '12px', height: '40px', padding: '0 14px',
          borderRadius: '20px', border: 'none', fontWeight: 700, cursor: 'pointer',
          background: scanner.torch ? '#f6e05e' : 'rgba(255,255,255,0.85)',
        }}>
        {scanner.torch ? 'Light on' : 'Light'}
      </button>
    )}
  </div>
);

export default CameraView;
