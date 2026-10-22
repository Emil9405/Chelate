// components/labels/ScanPage.js
// Scan a bottle label (Data Matrix) with the phone camera and act on the bottle.
//
// Live camera needs a secure context (HTTPS or localhost). Over plain HTTP
// browsers do not expose the camera stream, so the page falls back to
// "take a photo": the native camera app returns a picture, which is decoded
// here. The short code printed under each symbol can be typed instead, and a
// USB/Bluetooth scanner in keyboard mode types into the same field.

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';
import useIsMobile from '../hooks/useIsMobile';
import PlacementModal from '../Reagents/PlacementModal';
import BottleCard from './BottleCard';
import CameraView from './CameraView';
import useCameraScanner from './useCameraScanner';
import { ScanIcon } from './LabelIcons';
import { canActOnStock } from './permissions';

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);


const ScanPage = ({ user }) => {
  const isMobile = useIsMobile();
  const fileRef = useRef(null);
  const handlingRef = useRef(false);
  const cameraWasOn = useRef(false);

  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [matches, setMatches] = useState([]);
  const [current, setCurrent] = useState(null);
  const [manual, setManual] = useState('');
  const [placing, setPlacing] = useState(false);
  const [historyKey, setHistoryKey] = useState(0);

  const canAct = canActOnStock(user);

  const lookupRef = useRef(null);
  const scanner = useCameraScanner({ onCode: (text) => lookupRef.current && lookupRef.current(text) });
  const { liveSupported, cameraOn, start, stop, cameraError } = scanner;

  const lookup = useCallback(async (code) => {
    if (handlingRef.current) return;
    handlingRef.current = true;
    cameraWasOn.current = cameraWasOn.current || cameraOn;
    stop();
    setError('');
    setStatus('Looking up…');
    setMatches([]);
    setCurrent(null);
    try {
      const found = await api.resolveLabel(code);
      if (found.length === 1) setCurrent(found[0]);
      else setMatches(found);
      setStatus('');
    } catch (e) {
      setStatus('');
      setError(e.message || 'Lookup failed');
    } finally {
      handlingRef.current = false;
    }
  }, [stop, cameraOn]);
  lookupRef.current = lookup;

  // Phones open straight into the camera; on a computer the code field
  // (keyboard or USB scanner) comes first and the webcam starts on request.
  useEffect(() => {
    if (liveSupported && isMobile) start();
  }, [liveSupported, isMobile, start]);

  useEffect(() => { if (cameraOn) cameraWasOn.current = true; }, [cameraOn]);

  const onPhoto = async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = '';
    if (!file) return;
    stop();
    setError('');
    setCurrent(null);
    setMatches([]);
    setStatus('Reading the photo…');
    try {
      const texts = await scanner.decodeFile(file);
      if (texts.length > 0) {
        lookup(texts[0]);
      } else {
        setStatus('');
        setError('No label found in the photo. Hold the phone 10–15 cm from the label so that it fills the middle of the frame, with no glare.');
      }
    } catch (err) {
      setStatus('');
      setError(err.message || 'Could not read the photo');
    }
  };

  const refresh = async () => {
    if (!current) return;
    const found = await api.resolveLabel(current.payload);
    if (found.length === 1) setCurrent(found[0]);
    setHistoryKey(k => k + 1);
  };

  const scanNext = () => {
    setCurrent(null);
    setMatches([]);
    setError('');
    setStatus('');
    if (liveSupported && (isMobile || cameraWasOn.current)) start();
  };

  const submitManual = (e) => {
    e.preventDefault();
    if (manual.trim()) {
      lookup(manual.trim());
      setManual('');
    }
  };

  const idle = !current && matches.length === 0;
  const shownError = error || (idle ? cameraError : '');

  return (
    <div style={{ padding: isMobile ? '68px 12px 16px' : '96px 20px 32px', maxWidth: '560px', margin: '0 auto' }}>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: '10px', fontSize: isMobile ? '1.5rem' : '1.75rem', fontWeight: 800, color: '#1a365d', margin: '0 0 12px' }}>
        <ScanIcon size={26} color="#3182ce" /> Scan label
      </h1>

      <canvas ref={scanner.canvasRef} style={{ display: 'none' }} />
      <input ref={fileRef} type="file" accept="image/*" capture="environment" onChange={onPhoto} style={{ display: 'none' }} />

      {/* Kept mounted (hidden while a result is shown) so "Scan next" can reattach the stream */}
      {liveSupported && <CameraView scanner={scanner} hidden={!idle || (!isMobile && !cameraOn)} />}

      {!liveSupported && idle && (
        <div style={{ background: '#ebf8ff', border: '1px solid #bee3f8', color: '#2a4365', borderRadius: '12px', padding: '12px 14px', fontSize: '0.85rem', lineHeight: 1.5 }}>
          Live scanning needs the site to be opened over <b>HTTPS</b>. Here, take a photo of the label instead —
          it is read on this device.
        </div>
      )}

      {status && <div role="status" style={{ marginTop: '10px', color: '#4a5568', fontSize: '0.9rem', textAlign: 'center' }}>{status}</div>}
      {shownError && (
        <div role="alert" style={{ marginTop: '10px', background: '#fff5f5', border: '1px solid #fed7d7', color: '#c53030', borderRadius: '10px', padding: '10px 12px', fontSize: '0.875rem' }}>
          {shownError}
        </div>
      )}

      {idle && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', marginTop: '12px' }}>
          {liveSupported && !isMobile && !cameraOn && (
            <Button variant="secondary" onClick={start} style={{ height: '48px' }}>Use webcam</Button>
          )}
          <Button variant={liveSupported ? 'secondary' : 'primary'} onClick={() => fileRef.current && fileRef.current.click()} style={{ height: '48px' }}>
            {liveSupported ? (isMobile ? 'Take a photo instead' : 'Read a photo of the label') : 'Take a photo of the label'}
          </Button>
          <form onSubmit={submitManual} style={{ display: 'flex', gap: '8px' }}>
            <input
              value={manual} onChange={e => setManual(e.target.value)}
              placeholder="Code under the symbol, e.g. 24870DF5"
              aria-label="Label code"
              autoFocus={!isMobile}
              autoCapitalize="characters" autoComplete="off" spellCheck={false}
              style={{ flex: 1, minWidth: 0, height: '44px', padding: '0 12px', border: '1px solid #e2e8f0', borderRadius: '10px', fontFamily: 'monospace' }}
            />
            <Button type="submit" variant="secondary" disabled={!manual.trim()} style={{ height: '44px' }}>Find</Button>
          </form>
        </div>
      )}

      {matches.length > 1 && (
        <div style={{ marginTop: '12px' }}>
          <div style={{ fontSize: '0.875rem', color: '#4a5568', marginBottom: '8px' }}>
            Several bottles share this short code. Pick yours:
          </div>
          {matches.map(m => (
            <button key={m.container_id} type="button" onClick={() => { setCurrent(m); setMatches([]); }}
              style={{ width: '100%', textAlign: 'left', padding: '12px', marginBottom: '8px', border: '1px solid #e2e8f0', borderRadius: '10px', background: 'white', cursor: 'pointer' }}>
              <div style={{ fontWeight: 700, color: '#1a365d' }}>{m.reagent_name}</div>
              <div style={{ fontSize: '0.8rem', color: '#718096' }}>{m.batch_number} · #{m.sequence_number} · {fmtQty(m.quantity)} {m.unit}</div>
            </button>
          ))}
        </div>
      )}

      {current && (
        <div style={{ marginTop: '4px' }}>
          <BottleCard c={current} canAct={canAct} onChanged={refresh} onMove={() => setPlacing(true)} historyKey={historyKey} />
        </div>
      )}

      {(current || matches.length > 1 || error) && (
        <Button variant="primary" onClick={scanNext} style={{ width: '100%', height: '48px', marginTop: '12px' }}
          icon={<ScanIcon size={18} color="white" />}>
          Scan next
        </Button>
      )}

      {placing && current && (
        <PlacementModal
          isOpen={placing}
          containers={[{
            id: current.container_id, position_id: current.position_id, placement_id: current.placement_id,
            quantity: current.quantity, room_name: current.room_name, zone_name: current.zone_name,
            position_name: current.position_name,
          }]}
          action={current.position_id ? 'move' : 'place'}
          batchId={current.batch_id}
          onConfirm={async (assignments) => {
            for (const { containerIds, positionId } of assignments) {
              for (const id of containerIds) {
                if (current.position_id) await api.moveContainer(id, positionId, 'scan');
                else await api.placeContainer(id, { position_id: positionId, source: 'scan' });
              }
            }
            setPlacing(false);
            await refresh();
          }}
          onClose={() => setPlacing(false)}
        />
      )}
    </div>
  );
};

export default ScanPage;
