// components/labels/ScanPage.js
// Scan a Chelate label with the phone camera and act on what it is:
//   - a bottle (Data Matrix): contents, write-off, move, history;
//   - a place (QR on a room, cabinet or shelf): what is there, and on a shelf
//     "put bottles here" by scanning them.
// Scanning a bottle, then "Move: scan the shelf label", then the shelf's QR
// moves the bottle.
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
import LocationCard from './LocationCard';
import PlaceHere from './PlaceHere';
import CameraView from './CameraView';
import useCameraScanner from './useCameraScanner';
import { ScanIcon } from './LabelIcons';
import { canActOnStock } from './permissions';

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);

const bottleTitle = (c) => `${c.reagent_name} · ${c.batch_number} #${c.sequence_number}`;
const bottleWhere = (c) => (c.position_name ? [c.room_name, c.zone_name, c.position_name].filter(Boolean).join(' / ') : null);

const ScanPage = ({ user }) => {
  const isMobile = useIsMobile();
  const fileRef = useRef(null);
  const handlingRef = useRef(false);
  const cameraWasOn = useRef(false);

  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [matches, setMatches] = useState([]);
  const [current, setCurrent] = useState(null);
  const [location, setLocation] = useState(null);
  const [putHere, setPutHere] = useState(null);       // shelf being filled by scanning bottles
  const [moveFor, setMoveFor] = useState(null);       // bottle waiting for its shelf label
  const [confirmMove, setConfirmMove] = useState(null); // { bottle, shelf }
  const [moving, setMoving] = useState(false);
  const [manual, setManual] = useState('');
  const [placing, setPlacing] = useState(false);
  const [historyKey, setHistoryKey] = useState(0);

  const canAct = canActOnStock(user);

  const lookupRef = useRef(null);
  const scanner = useCameraScanner({ onCode: (text) => lookupRef.current && lookupRef.current(text) });
  const { liveSupported, cameraOn, start, stop, cameraError } = scanner;

  const clearResult = useCallback(() => {
    setMatches([]);
    setCurrent(null);
    setLocation(null);
    setConfirmMove(null);
  }, []);

  const restartCamera = useCallback(() => {
    if (liveSupported && (isMobile || cameraWasOn.current)) start();
  }, [liveSupported, isMobile, start]);

  const lookup = useCallback(async (code) => {
    if (handlingRef.current) return;
    handlingRef.current = true;
    cameraWasOn.current = cameraWasOn.current || cameraOn;
    stop();
    setError('');
    setNotice('');
    setStatus('Looking up…');
    clearResult();
    try {
      const r = await api.resolveCode(code);
      if (r.location) {
        if (moveFor) {
          if (r.location.kind === 'position') {
            setConfirmMove({ bottle: moveFor, shelf: r.location });
          } else {
            setError(`That is a ${r.location.kind === 'zone' ? 'cabinet' : 'room'} label. Scan the label of the shelf itself.`);
            restartCamera();
          }
        } else {
          setLocation(r.location);
        }
      } else if (moveFor) {
        setError('Scan the shelf label, not a bottle. Cancel to go back to the bottle.');
        restartCamera();
      } else if (r.matches.length === 1) {
        setCurrent(r.matches[0]);
      } else {
        setMatches(r.matches);
      }
      setStatus('');
    } catch (e) {
      setStatus('');
      setError(e.message || 'Lookup failed');
      if (moveFor) restartCamera();
    } finally {
      handlingRef.current = false;
    }
  }, [stop, cameraOn, moveFor, clearResult, restartCamera]);
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
    clearResult();
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
    const r = await api.resolveCode(current.payload);
    if (r.matches.length === 1) setCurrent(r.matches[0]);
    setHistoryKey(k => k + 1);
  };

  const scanNext = () => {
    clearResult();
    setMoveFor(null);
    setPutHere(null);
    setError('');
    setNotice('');
    setStatus('');
    restartCamera();
  };

  const startShelfScan = () => {
    setMoveFor(current);
    clearResult();
    setError('');
    setNotice('');
    restartCamera();
  };

  const cancelShelfScan = async () => {
    const bottle = moveFor;
    setMoveFor(null);
    setConfirmMove(null);
    setError('');
    stop();
    if (bottle) {
      try {
        const r = await api.resolveCode(bottle.payload);
        if (r.matches.length === 1) setCurrent(r.matches[0]);
      } catch (_) { setCurrent(bottle); }
    }
  };

  const performMove = async () => {
    const { bottle, shelf } = confirmMove;
    setMoving(true);
    setError('');
    try {
      if (bottle.position_id) await api.moveContainer(bottle.container_id, shelf.id, 'scan');
      else await api.placeContainer(bottle.container_id, { position_id: shelf.id, source: 'scan' });
      const r = await api.resolveCode(bottle.payload);
      setMoveFor(null);
      setConfirmMove(null);
      if (r.matches.length === 1) setCurrent(r.matches[0]);
      setHistoryKey(k => k + 1);
      setNotice(`${bottle.position_id ? 'Moved' : 'Placed'} to ${shelf.path}`);
    } catch (e) {
      setError(e.message || 'Could not move the bottle');
    } finally {
      setMoving(false);
    }
  };

  const finishPutHere = async (count) => {
    const shelf = putHere;
    setPutHere(null);
    setError('');
    try {
      const r = await api.resolveCode(shelf.payload);
      if (r.location) setLocation(r.location);
    } catch (_) { setLocation(shelf); }
    if (count > 0) setNotice(`${count} bottle${count !== 1 ? 's' : ''} placed on ${shelf.path}`);
  };

  const submitManual = (e) => {
    e.preventDefault();
    if (manual.trim()) {
      lookup(manual.trim());
      setManual('');
    }
  };

  const idle = !current && matches.length === 0 && !location && !confirmMove && !putHere;
  const shownError = error || (idle ? cameraError : '');
  const hasResult = current || matches.length > 1 || location || confirmMove;

  return (
    <div style={{ padding: isMobile ? '68px 12px 16px' : '96px 20px 32px', maxWidth: '560px', margin: '0 auto' }}>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: '10px', fontSize: isMobile ? '1.5rem' : '1.75rem', fontWeight: 800, color: '#1a365d', margin: '0 0 12px' }}>
        <ScanIcon size={26} color="#3182ce" /> Scan label
      </h1>

      <canvas ref={scanner.canvasRef} style={{ display: 'none' }} />
      <input ref={fileRef} type="file" accept="image/*" capture="environment" onChange={onPhoto} style={{ display: 'none' }} />

      {putHere ? (
        <PlaceHere shelf={putHere} onSwitchShelf={setPutHere} onDone={finishPutHere} />
      ) : (
        <>
          {moveFor && (
            <div style={{ background: '#faf5ff', border: '2px solid #805ad5', borderRadius: '12px', padding: '10px 14px', marginBottom: '10px' }}>
              <div style={{ fontSize: '0.7rem', fontWeight: 800, letterSpacing: '0.06em', color: '#6b46c1' }}>
                {moveFor.position_id ? 'MOVING' : 'PLACING'}
              </div>
              <div style={{ fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere' }}>{bottleTitle(moveFor)}</div>
              {!confirmMove && (
                <div style={{ fontSize: '0.8rem', color: '#4a5568', marginTop: '2px' }}>Scan the label of the shelf where it goes.</div>
              )}
              {!confirmMove && (
                <button type="button" onClick={cancelShelfScan}
                  style={{ background: 'none', border: 'none', color: '#6b46c1', fontWeight: 700, padding: '6px 0 0', cursor: 'pointer' }}>
                  Cancel
                </button>
              )}
            </div>
          )}

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
          {notice && (
            <div role="status" style={{ marginTop: '10px', background: '#f0fff4', border: '1px solid #c6f6d5', color: '#276749', borderRadius: '10px', padding: '10px 12px', fontSize: '0.875rem' }}>
              ✓ {notice}
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
                  placeholder="Code under the symbol, e.g. 24870DF5 or P1A2B3C4D"
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
              <BottleCard c={current} canAct={canAct} onChanged={refresh} onMove={() => setPlacing(true)}
                onScanShelf={canAct ? startShelfScan : undefined} historyKey={historyKey} />
            </div>
          )}

          {location && (
            <div style={{ marginTop: '4px' }}>
              <LocationCard loc={location} canAct={canAct} onPutHere={() => { setPutHere(location); setLocation(null); setNotice(''); }} />
            </div>
          )}

          {confirmMove && (
            <div style={{ background: 'white', border: '2px solid #805ad5', borderRadius: '14px', padding: '14px 16px' }}>
              <div style={{ fontSize: '0.9rem', color: '#2d3748', lineHeight: 1.5 }}>
                {confirmMove.bottle.position_id ? 'Move' : 'Place'} <b>{bottleTitle(confirmMove.bottle)}</b>
                {confirmMove.bottle.position_id && <> from <b>{bottleWhere(confirmMove.bottle)}</b></>}
                {' '}to <b>{confirmMove.shelf.path}</b>?
              </div>
              <div style={{ display: 'flex', gap: '8px', marginTop: '12px' }}>
                <Button variant="primary" onClick={performMove} disabled={moving} loading={moving} style={{ flex: 1, height: '46px' }}>
                  {confirmMove.bottle.position_id ? 'Move here' : 'Place here'}
                </Button>
                <Button variant="secondary" onClick={cancelShelfScan} disabled={moving} style={{ height: '46px' }}>Cancel</Button>
              </div>
            </div>
          )}

          {hasResult && !confirmMove && (
            <Button variant="primary" onClick={scanNext} style={{ width: '100%', height: '48px', marginTop: '12px' }}
              icon={<ScanIcon size={18} color="white" />}>
              Scan next
            </Button>
          )}
          {error && !hasResult && !moveFor && (
            <Button variant="primary" onClick={scanNext} style={{ width: '100%', height: '48px', marginTop: '12px' }}
              icon={<ScanIcon size={18} color="white" />}>
              Scan next
            </Button>
          )}
        </>
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
