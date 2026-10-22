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
import { createDataMatrixDecoder } from './barcode';
import { ScanIcon } from './LabelIcons';

const SCAN_INTERVAL_MS = 180;
const GUIDE = 0.7; // share of the visible square that is decoded

const fmtQty = (q) => String(Math.round(Number(q) * 1000) / 1000);
const fmtDate = (v) => (v ? new Date(v).toLocaleDateString('ru-RU') : '—');

const isExpired = (c) => c.batch_status === 'expired'
  || (c.expiry_date && new Date(c.expiry_date).getTime() < Date.now());

const STATUS = {
  full: { text: 'Sealed', bg: '#c6f6d5', fg: '#22543d' },
  partial: { text: 'Opened', bg: '#fefcbf', fg: '#744210' },
  empty: { text: 'Empty', bg: '#fed7d7', fg: '#822727' },
  disposed: { text: 'Disposed', bg: '#e2e8f0', fg: '#4a5568' },
};

const ghsList = (value) => (value || '')
  .split(',').map(c => c.trim().toUpperCase()).filter(c => /^GHS0[1-9]$/.test(c));

/** Image file -> something drawImage accepts, with width/height */
const loadImage = async (file) => {
  if (typeof createImageBitmap === 'function') {
    try { return await createImageBitmap(file); } catch (_) { /* fall through */ }
  }
  const url = URL.createObjectURL(file);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    return { source: img, width: img.naturalWidth, height: img.naturalHeight };
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
};

// ==================== RESULT CARD ====================

const BottleCard = ({ c, canAct, onChanged, onMove }) => {
  const [qty, setQty] = useState('');
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState(null);
  const expired = isExpired(c);
  const usable = c.container_status !== 'empty' && c.container_status !== 'disposed' && c.quantity > 0;
  const st = STATUS[c.container_status] || { text: c.container_status, bg: '#edf2f7', fg: '#2d3748' };
  const pct = c.original_quantity > 0 ? Math.max(0, Math.min(100, (c.quantity / c.original_quantity) * 100)) : 0;
  const location = c.position_name
    ? [c.room_name, c.zone_name, c.position_name].filter(Boolean).join(' / ')
    : null;

  const use = async () => {
    const n = parseFloat(String(qty).replace(',', '.'));
    if (!(n > 0)) { setMsg({ error: true, text: 'Enter an amount' }); return; }
    if (expired && !reason.trim()) { setMsg({ error: true, text: 'Expired batch: enter a reason' }); return; }
    setBusy(true);
    setMsg(null);
    try {
      const res = await api.useFromContainer(c.container_id, {
        quantity: n,
        ...(expired ? { allow_expired: true, expired_reason: reason.trim() } : {}),
      });
      const d = res?.data || {};
      setMsg({ text: `Used ${fmtQty(n)} ${c.unit} — ${fmtQty(d.container_remaining ?? 0)} ${c.unit} left in this bottle` });
      setQty('');
      setReason('');
      await onChanged();
    } catch (e) {
      setMsg({ error: true, text: e.message || 'Write-off failed' });
    } finally {
      setBusy(false);
    }
  };

  const row = { display: 'flex', justifyContent: 'space-between', gap: '12px', padding: '6px 0', borderBottom: '1px solid #f1f5f9', fontSize: '0.875rem' };
  const key = { color: '#718096', flexShrink: 0 };
  const val = { color: '#1a202c', fontWeight: 500, textAlign: 'right', overflowWrap: 'anywhere' };

  return (
    <div style={{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '14px', padding: '16px', boxShadow: '0 2px 10px rgba(26,54,93,0.06)' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '10px', alignItems: 'flex-start' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '1.15rem', fontWeight: 800, color: '#1a365d', overflowWrap: 'anywhere' }}>{c.reagent_name}</div>
          <div style={{ fontSize: '0.8rem', color: '#718096', marginTop: '2px' }}>
            {[c.cas_number && `CAS ${c.cas_number}`, c.formula].filter(Boolean).join(' · ') || ' '}
          </div>
        </div>
        <span style={{ background: st.bg, color: st.fg, fontWeight: 700, fontSize: '0.75rem', padding: '3px 10px', borderRadius: '999px', whiteSpace: 'nowrap' }}>
          {st.text}
        </span>
      </div>

      {ghsList(c.hazard_pictograms).length > 0 && (
        <div style={{ display: 'flex', gap: '6px', marginTop: '10px' }}>
          {ghsList(c.hazard_pictograms).map(g => (
            <img key={g} src={`/assets/ghs/${g}.svg`} alt={g} title={g} style={{ width: '34px', height: '34px' }} />
          ))}
        </div>
      )}

      <div style={{ marginTop: '12px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.875rem', marginBottom: '4px' }}>
          <span style={{ color: '#718096' }}>In this bottle</span>
          <span style={{ fontWeight: 700, color: '#1a202c' }}>{fmtQty(c.quantity)} / {fmtQty(c.original_quantity)} {c.unit}</span>
        </div>
        <div style={{ height: '6px', background: '#edf2f7', borderRadius: '3px', overflow: 'hidden' }}>
          <div style={{ width: `${pct}%`, height: '100%', background: pct <= 20 ? '#e53e3e' : '#38a169' }} />
        </div>
      </div>

      <div style={{ marginTop: '10px' }}>
        <div style={row}><span style={key}>Batch</span><span style={val}>{c.batch_number} · bottle #{c.sequence_number} of {c.batch_container_count}</span></div>
        <div style={row}><span style={key}>Location</span><span style={{ ...val, color: location ? '#1a202c' : '#a0aec0' }}>{location || 'Not placed'}</span></div>
        <div style={row}>
          <span style={key}>Expiry</span>
          <span style={{ ...val, color: expired ? '#c53030' : '#1a202c', fontWeight: expired ? 700 : 500 }}>
            {fmtDate(c.expiry_date)}{expired ? ' — expired' : ''}
          </span>
        </div>
        <div style={{ ...row, borderBottom: 'none' }}><span style={key}>Label code</span><span style={{ ...val, fontFamily: 'monospace' }}>{c.short_code}</span></div>
      </div>

      {canAct && usable && (
        <div style={{ marginTop: '12px', paddingTop: '12px', borderTop: '1px solid #e2e8f0', display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {expired && (
            <input
              type="text" value={reason} onChange={e => setReason(e.target.value)} maxLength={500}
              placeholder="Reason for using expired stock (required)"
              style={{ height: '44px', padding: '0 12px', border: '1px solid #fbd38d', borderRadius: '10px', background: '#fffaf0' }}
            />
          )}
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <div style={{ position: 'relative', flex: 1, minWidth: 0 }}>
              <input
                type="number" inputMode="decimal" step="any" min="0" value={qty}
                onChange={e => setQty(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') use(); }}
                placeholder={`Amount, max ${fmtQty(c.quantity)}`}
                aria-label={`Amount in ${c.unit}`}
                style={{ width: '100%', height: '44px', padding: '0 44px 0 12px', border: '1px solid #e2e8f0', borderRadius: '10px', boxSizing: 'border-box' }}
              />
              <span style={{ position: 'absolute', right: '12px', top: '50%', transform: 'translateY(-50%)', color: '#718096', fontWeight: 600, fontSize: '0.85rem', pointerEvents: 'none' }}>
                {c.unit}
              </span>
            </div>
            <Button variant="primary" onClick={use} disabled={busy} loading={busy}
              style={{ height: '44px', background: expired ? '#dd6b20' : '#38a169', flexShrink: 0 }}>
              Use
            </Button>
          </div>
          <Button variant="secondary" onClick={onMove} style={{ height: '44px' }}>
            {c.position_id ? 'Move to another shelf' : 'Place on a shelf'}
          </Button>
        </div>
      )}

      {msg && (
        <div role="status" style={{
          marginTop: '10px', padding: '8px 10px', borderRadius: '8px', fontSize: '0.85rem',
          background: msg.error ? '#fff5f5' : '#f0fff4', color: msg.error ? '#c53030' : '#276749',
          border: `1px solid ${msg.error ? '#fed7d7' : '#c6f6d5'}`,
        }}>
          {msg.error ? '⚠ ' : '✓ '}{msg.text}
        </div>
      )}
    </div>
  );
};

// ==================== PAGE ====================

const ScanPage = ({ user }) => {
  const isMobile = useIsMobile();
  const liveSupported = typeof window !== 'undefined' && window.isSecureContext
    && !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia);

  const videoRef = useRef(null);
  const canvasRef = useRef(null);
  const streamRef = useRef(null);
  const timerRef = useRef(null);
  const decoderRef = useRef(null);
  const fileRef = useRef(null);
  const handlingRef = useRef(false);
  const mountedRef = useRef(true);
  const streamWasOn = useRef(false);

  const [cameraOn, setCameraOn] = useState(false);
  const [torch, setTorch] = useState(null); // null = unsupported
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [matches, setMatches] = useState([]);
  const [current, setCurrent] = useState(null);
  const [manual, setManual] = useState('');
  const [placing, setPlacing] = useState(false);

  const role = (user?.role || '').toString().toLowerCase();
  const perms = user?.permissions;
  const hasPerm = (k) => (Array.isArray(perms) ? perms.includes(k) : !!(perms && perms[k] === true));
  const canAct = role === 'admin' || role === 'researcher' || hasPerm('edit_batch');

  const getDecoder = useCallback(async () => {
    if (!decoderRef.current) decoderRef.current = await createDataMatrixDecoder();
    return decoderRef.current;
  }, []);

  const stopCamera = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = null;
    if (streamRef.current) streamRef.current.getTracks().forEach(t => t.stop());
    streamRef.current = null;
    setCameraOn(false);
    setTorch(null);
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; stopCamera(); };
  }, [stopCamera]);

  const lookup = useCallback(async (code) => {
    if (handlingRef.current) return;
    handlingRef.current = true;
    streamWasOn.current = !!streamRef.current;
    stopCamera();
    if (navigator.vibrate) navigator.vibrate(60);
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
  }, [stopCamera]);

  const scanFrame = useCallback(async () => {
    const video = videoRef.current;
    const canvas = canvasRef.current;
    if (!streamRef.current || !video || !canvas) return;
    try {
      if (video.readyState >= 2 && video.videoWidth > 0) {
        // The preview shows the centre square of the frame (object-fit: cover);
        // decode the guide area inside it.
        const side = Math.min(video.videoWidth, video.videoHeight) * GUIDE;
        const sx = (video.videoWidth - side) / 2;
        const sy = (video.videoHeight - side) / 2;
        const size = Math.round(Math.min(side, 800));
        canvas.width = size;
        canvas.height = size;
        canvas.getContext('2d', { willReadFrequently: true }).drawImage(video, sx, sy, side, side, 0, 0, size, size);
        const decoder = await getDecoder();
        const texts = await decoder.decode(canvas);
        if (texts.length > 0 && streamRef.current) {
          lookup(texts[0]);
          return;
        }
      }
    } catch (e) {
      // keep scanning; a single bad frame is not an error worth showing
    }
    if (streamRef.current) timerRef.current = setTimeout(scanFrame, SCAN_INTERVAL_MS);
  }, [lookup, getDecoder]);

  const startCamera = useCallback(async () => {
    setError('');
    setCurrent(null);
    setMatches([]);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: false,
        video: { facingMode: { ideal: 'environment' }, width: { ideal: 1280 }, height: { ideal: 720 } },
      });
      if (!mountedRef.current) {
        // Left the page while the permission prompt was open: release the camera
        stream.getTracks().forEach(t => t.stop());
        return;
      }
      streamRef.current = stream;
      const video = videoRef.current;
      video.srcObject = stream;
      await video.play().catch(() => {});
      setCameraOn(true);
      const track = stream.getVideoTracks()[0];
      const caps = track && track.getCapabilities ? track.getCapabilities() : {};
      setTorch(caps && caps.torch ? false : null);
      setStatus('Point the camera at the label');
      getDecoder().catch(() => {}); // load the decoder while the camera warms up
      timerRef.current = setTimeout(scanFrame, 300);
    } catch (e) {
      stopCamera();
      setStatus('');
      setError(e && e.name === 'NotAllowedError'
        ? 'Camera access was denied. Allow it in the browser settings, or take a photo instead.'
        : e && e.name === 'NotFoundError'
          ? 'No camera found on this device. Take a photo or type the code.'
          : `Camera unavailable (${e.message || e.name}). Take a photo instead.`);
    }
  }, [scanFrame, stopCamera, getDecoder]);

  // Phones open straight into the camera; on a computer the code field
  // (keyboard or USB scanner) comes first and the webcam starts on request.
  useEffect(() => {
    if (liveSupported && isMobile) startCamera();
  }, [liveSupported, isMobile, startCamera]);

  const toggleTorch = async () => {
    const track = streamRef.current && streamRef.current.getVideoTracks()[0];
    if (!track) return;
    try {
      await track.applyConstraints({ advanced: [{ torch: !torch }] });
      setTorch(!torch);
    } catch (_) { setTorch(null); }
  };

  const onPhoto = async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = '';
    if (!file) return;
    stopCamera();
    setError('');
    setCurrent(null);
    setMatches([]);
    setStatus('Reading the photo…');
    try {
      const img = await loadImage(file);
      const source = img.source || img;
      const canvas = canvasRef.current;
      const decoder = await getDecoder();
      const attempt = async (sx, sy, sw, sh, maxSide) => {
        const k = Math.min(1, maxSide / Math.max(sw, sh));
        canvas.width = Math.round(sw * k);
        canvas.height = Math.round(sh * k);
        canvas.getContext('2d', { willReadFrequently: true }).drawImage(source, sx, sy, sw, sh, 0, 0, canvas.width, canvas.height);
        return decoder.decode(canvas);
      };
      let texts = await attempt(0, 0, img.width, img.height, 1600);
      if (texts.length === 0) {
        // Small label far away: look at the middle at full resolution
        const s = Math.min(img.width, img.height) * 0.5;
        texts = await attempt((img.width - s) / 2, (img.height - s) / 2, s, s, 2000);
      }
      if (img.close) img.close();
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
  };

  const scanNext = () => {
    setCurrent(null);
    setMatches([]);
    setError('');
    setStatus('');
    if (liveSupported && (isMobile || streamWasOn.current)) startCamera();
  };

  const submitManual = (e) => {
    e.preventDefault();
    if (manual.trim()) {
      lookup(manual.trim());
      setManual('');
    }
  };

  const showCamera = liveSupported && !current && matches.length === 0;

  return (
    <div style={{ padding: isMobile ? '68px 12px 16px' : '96px 20px 32px', maxWidth: '560px', margin: '0 auto' }}>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: '10px', fontSize: isMobile ? '1.5rem' : '1.75rem', fontWeight: 800, color: '#1a365d', margin: '0 0 12px' }}>
        <ScanIcon size={26} color="#3182ce" /> Scan label
      </h1>

      <canvas ref={canvasRef} style={{ display: 'none' }} />
      <input ref={fileRef} type="file" accept="image/*" capture="environment" onChange={onPhoto} style={{ display: 'none' }} />

      {/* Kept mounted (hidden while a result is shown) so "Scan next" can reattach the stream */}
      {liveSupported && (
        <div style={{ position: 'relative', width: '100%', aspectRatio: '1 / 1', background: '#1a202c', borderRadius: '16px', overflow: 'hidden', display: showCamera && (isMobile || cameraOn) ? 'block' : 'none' }}>
          <video ref={videoRef} playsInline muted autoPlay
            style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />
          <div aria-hidden="true" style={{
            position: 'absolute', left: `${(1 - GUIDE) * 50}%`, top: `${(1 - GUIDE) * 50}%`,
            width: `${GUIDE * 100}%`, height: `${GUIDE * 100}%`,
            border: '3px solid rgba(255,255,255,0.9)', borderRadius: '12px',
            boxShadow: '0 0 0 9999px rgba(0,0,0,0.35)',
          }} />
          {!cameraOn && (
            <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <Button variant="primary" onClick={startCamera}>Start camera</Button>
            </div>
          )}
          {torch !== null && cameraOn && (
            <button type="button" onClick={toggleTorch} aria-pressed={torch}
              style={{ position: 'absolute', right: '12px', bottom: '12px', height: '40px', padding: '0 14px', borderRadius: '20px', border: 'none', background: torch ? '#f6e05e' : 'rgba(255,255,255,0.85)', fontWeight: 700, cursor: 'pointer' }}>
              {torch ? 'Light on' : 'Light'}
            </button>
          )}
        </div>
      )}

      {!liveSupported && !current && matches.length === 0 && (
        <div style={{ background: '#ebf8ff', border: '1px solid #bee3f8', color: '#2a4365', borderRadius: '12px', padding: '12px 14px', fontSize: '0.85rem', lineHeight: 1.5 }}>
          Live scanning needs the site to be opened over <b>HTTPS</b>. Here, take a photo of the label instead —
          it is read on this device.
        </div>
      )}

      {status && <div role="status" style={{ marginTop: '10px', color: '#4a5568', fontSize: '0.9rem', textAlign: 'center' }}>{status}</div>}
      {error && (
        <div role="alert" style={{ marginTop: '10px', background: '#fff5f5', border: '1px solid #fed7d7', color: '#c53030', borderRadius: '10px', padding: '10px 12px', fontSize: '0.875rem' }}>
          {error}
        </div>
      )}

      {!current && matches.length === 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', marginTop: '12px' }}>
          {liveSupported && !isMobile && !cameraOn && (
            <Button variant="secondary" onClick={startCamera} style={{ height: '48px' }}>Use webcam</Button>
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
          <BottleCard c={current} canAct={canAct} onChanged={refresh} onMove={() => setPlacing(true)} />
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
                if (current.position_id) await api.moveContainer(id, positionId);
                else await api.placeContainer(id, { position_id: positionId });
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
