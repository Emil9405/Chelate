// components/labels/PlaceHere.js
// Put bottles on a shelf by scanning them one after another.
//
// Each bottle is placed (or moved, if it is recorded on another shelf) onto the
// shelf with source "scan", so the move lands in the bottle's history.
// Scanning another shelf's QR label switches the target shelf.

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import useIsMobile from '../hooks/useIsMobile';
import CameraView from './CameraView';
import useCameraScanner from './useCameraScanner';

const TONE = {
  ok: { bg: '#f0fff4', fg: '#276749', border: '#c6f6d5', flash: '#48bb78' },
  same: { bg: '#ebf8ff', fg: '#2a4365', border: '#bee3f8', flash: '#4299e1' },
  warn: { bg: '#fffaf0', fg: '#7b341e', border: '#fbd38d', flash: '#ed8936' },
  error: { bg: '#fff5f5', fg: '#c53030', border: '#fed7d7', flash: '#f56565' },
};

const title = (c) => `${c.reagent_name} · ${c.batch_number} #${c.sequence_number}`;
const where = (c) => (c.position_name ? [c.room_name, c.zone_name, c.position_name].filter(Boolean).join(' / ') : null);

const PlaceHere = ({ shelf, onSwitchShelf, onDone }) => {
  const isMobile = useIsMobile();
  const fileRef = useRef(null);
  const busyRef = useRef(false);
  const shelfRef = useRef(shelf);
  shelfRef.current = shelf;
  const [lines, setLines] = useState([]);
  const [busy, setBusy] = useState(false);
  const [manual, setManual] = useState('');

  const push = useCallback((tone, text, sub) => {
    setLines(prev => [{ id: Date.now() + Math.random(), tone, text, sub }, ...prev].slice(0, 60));
  }, []);

  const handle = useCallback(async (code) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    try {
      const current = shelfRef.current;
      const r = await api.resolveCode(code);

      if (r.location) {
        if (r.location.kind === 'position') {
          if (r.location.id !== current.id) {
            onSwitchShelf(r.location);
            push('same', `Now putting bottles on ${r.location.path}`);
          } else {
            push('same', 'That is the shelf you are already filling');
          }
        } else {
          push('warn', 'That is a cabinet or room label', 'Scan a shelf label to change the shelf, or a bottle to put it here.');
        }
        return;
      }

      if (r.matches.length !== 1) {
        push('error', 'Several bottles share this code', 'Scan the symbol on the bottle instead of typing the short code.');
        return;
      }
      const c = r.matches[0];
      if (c.container_status === 'empty' || c.container_status === 'disposed') {
        push('warn', title(c), `Recorded as ${c.container_status}: not placed.`);
        return;
      }
      if (c.position_id === current.id) {
        push('same', title(c), 'Already on this shelf.');
        return;
      }
      if (c.position_id) {
        await api.moveContainer(c.container_id, current.id, 'scan');
        push('ok', title(c), `Moved here from ${where(c)}`);
      } else {
        await api.placeContainer(c.container_id, { position_id: current.id, source: 'scan' });
        push('ok', title(c), 'Placed here (it had no shelf).');
      }
      if (navigator.vibrate) navigator.vibrate(60);
    } catch (e) {
      push('error', 'Not placed', e.message || 'Request failed');
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, [onSwitchShelf, push]);

  const scanner = useCameraScanner({ onCode: handle, continuous: true, cooldownMs: 3000 });
  const { liveSupported, start, stop, cameraOn } = scanner;

  useEffect(() => {
    if (liveSupported && isMobile) start();
    return () => stop();
  }, [liveSupported, isMobile, start, stop]);

  const onPhoto = async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = '';
    if (!file) return;
    try {
      const texts = await scanner.decodeFile(file);
      if (texts.length > 0) await handle(texts[0]);
      else push('error', 'No label found in the photo', 'Hold the phone 10–15 cm from the label, without glare.');
    } catch (err) {
      push('error', 'Could not read the photo', err.message);
    }
  };

  const placedCount = lines.filter(l => l.tone === 'ok').length;
  const last = lines[0];
  const flash = last ? TONE[last.tone].flash : null;

  return (
    <div>
      <div style={{ background: '#f0fff4', border: '2px solid #38a169', borderRadius: '12px', padding: '10px 14px', marginBottom: '10px' }}>
        <div style={{ fontSize: '0.7rem', fontWeight: 800, letterSpacing: '0.06em', color: '#2f855a' }}>PUTTING BOTTLES ON</div>
        <div style={{ fontSize: '1.05rem', fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere' }}>{shelf.path}</div>
        <div style={{ fontSize: '0.78rem', color: '#4a5568', marginTop: '2px' }}>
          Scan a bottle: it is placed here, or moved here from another shelf. Scan another shelf's QR to change the shelf.
        </div>
      </div>

      <canvas ref={scanner.canvasRef} style={{ display: 'none' }} />
      <input ref={fileRef} type="file" accept="image/*" capture="environment" onChange={onPhoto} style={{ display: 'none' }} />
      {liveSupported && <CameraView scanner={scanner} hidden={!isMobile && !cameraOn} flash={flash} />}
      {!liveSupported && (
        <div style={{ background: '#ebf8ff', border: '1px solid #bee3f8', color: '#2a4365', borderRadius: '12px', padding: '10px 12px', fontSize: '0.82rem' }}>
          Live scanning needs HTTPS. Take a photo of each label instead, or type its code.
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '10px' }}>
        {liveSupported && !isMobile && !cameraOn && (
          <Button variant="secondary" onClick={start} style={{ height: '44px' }}>Use webcam</Button>
        )}
        <Button variant={liveSupported ? 'secondary' : 'primary'} onClick={() => fileRef.current && fileRef.current.click()} style={{ height: '44px' }} disabled={busy}>
          {liveSupported ? (isMobile ? 'Take a photo instead' : 'Read a photo of the label') : 'Take a photo of the label'}
        </Button>
        <form onSubmit={e => { e.preventDefault(); if (manual.trim()) { handle(manual.trim()); setManual(''); } }} style={{ display: 'flex', gap: '8px' }}>
          <input value={manual} onChange={e => setManual(e.target.value)} placeholder="Code under the symbol"
            aria-label="Label code" autoFocus={!isMobile} autoComplete="off" spellCheck={false}
            style={{ flex: 1, minWidth: 0, height: '44px', padding: '0 12px', border: '1px solid #e2e8f0', borderRadius: '10px', fontFamily: 'monospace' }} />
          <Button type="submit" variant="secondary" disabled={!manual.trim() || busy} style={{ height: '44px' }}>Add</Button>
        </form>
      </div>

      {lines.length > 0 && (
        <ul aria-label="Scanned bottles" style={{ listStyle: 'none', margin: '12px 0 0', padding: 0, display: 'flex', flexDirection: 'column', gap: '6px' }}>
          {lines.map((l, i) => {
            const t = TONE[l.tone];
            return (
              <li key={l.id} role={i === 0 ? 'status' : undefined} style={{ background: t.bg, color: t.fg, border: `1px solid ${t.border}`, borderRadius: '10px', padding: '8px 10px', fontSize: '0.85rem' }}>
                <div style={{ fontWeight: 700, overflowWrap: 'anywhere' }}>{l.tone === 'ok' ? '✓ ' : l.tone === 'error' ? '⚠ ' : ''}{l.text}</div>
                {l.sub && <div style={{ fontSize: '0.78rem', overflowWrap: 'anywhere' }}>{l.sub}</div>}
              </li>
            );
          })}
        </ul>
      )}

      <Button variant="primary" onClick={() => { stop(); onDone(placedCount); }} style={{ width: '100%', height: '48px', marginTop: '14px' }}>
        Done{placedCount > 0 ? ` — ${placedCount} bottle${placedCount !== 1 ? 's' : ''} placed` : ''}
      </Button>
    </div>
  );
};

export default PlaceHere;
