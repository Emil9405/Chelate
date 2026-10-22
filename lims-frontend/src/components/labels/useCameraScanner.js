// components/labels/useCameraScanner.js
// Camera + Data Matrix decoding, shared by the Scan page and inventory.
//
// Live camera needs a secure context (HTTPS or localhost). Elsewhere
// `liveSupported` is false and callers offer `decodeFile` (a photo taken with
// the native camera app) instead.
//
// continuous = false: stops after the first code (Scan page).
// continuous = true: keeps scanning; the same code is reported again only
// after `cooldownMs`, so a label held in view is not counted many times.

import { useCallback, useEffect, useRef, useState } from 'react';
import { createDataMatrixDecoder } from './barcode';

const SCAN_INTERVAL_MS = 180;
export const GUIDE = 0.7; // share of the visible square that is decoded

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

const useCameraScanner = ({ onCode, continuous = false, cooldownMs = 2500 } = {}) => {
  const liveSupported = typeof window !== 'undefined' && window.isSecureContext
    && !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia);

  const videoRef = useRef(null);
  const canvasRef = useRef(null);
  const streamRef = useRef(null);
  const timerRef = useRef(null);
  const decoderRef = useRef(null);
  const mountedRef = useRef(true);
  const pausedRef = useRef(false);
  const lastRef = useRef({ text: null, at: 0 });
  const onCodeRef = useRef(onCode);
  onCodeRef.current = onCode;

  const [cameraOn, setCameraOn] = useState(false);
  const [torch, setTorch] = useState(null); // null = unsupported
  const [cameraError, setCameraError] = useState('');

  const getDecoder = useCallback(async () => {
    if (!decoderRef.current) decoderRef.current = await createDataMatrixDecoder();
    return decoderRef.current;
  }, []);

  const stop = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = null;
    if (streamRef.current) streamRef.current.getTracks().forEach(t => t.stop());
    streamRef.current = null;
    if (mountedRef.current) {
      setCameraOn(false);
      setTorch(null);
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; stop(); };
  }, [stop]);

  const emit = useCallback((text) => {
    const now = Date.now();
    if (continuous && lastRef.current.text === text && now - lastRef.current.at < cooldownMs) {
      return false;
    }
    lastRef.current = { text, at: now };
    if (navigator.vibrate) navigator.vibrate(60);
    if (!continuous) stop();
    onCodeRef.current && onCodeRef.current(text);
    return true;
  }, [continuous, cooldownMs, stop]);

  const tick = useCallback(async () => {
    const video = videoRef.current;
    const canvas = canvasRef.current;
    if (!streamRef.current || !video || !canvas) return;
    try {
      if (!pausedRef.current && video.readyState >= 2 && video.videoWidth > 0) {
        // The preview shows the centre square of the frame (object-fit: cover);
        // decode the guide area inside it.
        const side = Math.min(video.videoWidth, video.videoHeight) * GUIDE;
        const sx = (video.videoWidth - side) / 2;
        const sy = (video.videoHeight - side) / 2;
        const size = Math.round(Math.min(side, 800));
        canvas.width = size;
        canvas.height = size;
        canvas.getContext('2d', { willReadFrequently: true }).drawImage(video, sx, sy, side, side, 0, 0, size, size);
        const texts = await (await getDecoder()).decode(canvas);
        if (texts.length > 0 && streamRef.current && emit(texts[0]) && !continuous) return;
      }
    } catch (_) {
      // keep scanning; a single bad frame is not an error worth showing
    }
    if (streamRef.current) timerRef.current = setTimeout(tick, SCAN_INTERVAL_MS);
  }, [getDecoder, emit, continuous]);

  const start = useCallback(async () => {
    if (streamRef.current) return;
    setCameraError('');
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
      if (video) {
        video.srcObject = stream;
        await video.play().catch(() => {});
      }
      setCameraOn(true);
      const track = stream.getVideoTracks()[0];
      const caps = track && track.getCapabilities ? track.getCapabilities() : {};
      setTorch(caps && caps.torch ? false : null);
      getDecoder().catch(() => {}); // load the decoder while the camera warms up
      timerRef.current = setTimeout(tick, 300);
    } catch (e) {
      stop();
      setCameraError(e && e.name === 'NotAllowedError'
        ? 'Camera access was denied. Allow it in the browser settings, or take a photo instead.'
        : e && e.name === 'NotFoundError'
          ? 'No camera found on this device. Take a photo or type the code.'
          : `Camera unavailable (${(e && (e.message || e.name)) || 'error'}). Take a photo instead.`);
    }
  }, [getDecoder, stop, tick]);

  const toggleTorch = useCallback(async () => {
    const track = streamRef.current && streamRef.current.getVideoTracks()[0];
    if (!track) return;
    try {
      await track.applyConstraints({ advanced: [{ torch: !torch }] });
      setTorch(!torch);
    } catch (_) { setTorch(null); }
  }, [torch]);

  /** Pause decoding (camera stays on), e.g. while a result is being handled */
  const pause = useCallback((value) => { pausedRef.current = value; }, []);

  /** Decodes a photo; returns the codes found (empty array if none) */
  const decodeFile = useCallback(async (file) => {
    const img = await loadImage(file);
    const source = img.source || img;
    const canvas = canvasRef.current || document.createElement('canvas');
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
    return texts;
  }, [getDecoder]);

  return {
    videoRef, canvasRef, liveSupported, cameraOn, torch, cameraError, setCameraError,
    start, stop, pause, toggleTorch, decodeFile,
  };
};

export default useCameraScanner;
