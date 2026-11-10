// components/hooks/useFileUrl.js
// Images behind the session token.
//
// <img src> cannot send the Authorization header, which is why equipment
// photos did not show through the protected API and the files were made
// public. Here the file is fetched with the token and handed to <img> as a
// blob: URL.

import { useEffect, useState } from 'react';
import { api } from '../../services/api';

// url -> Promise<blob URL>. Shared by every component, so a photo shown in
// the list, the card and the zoom preview is downloaded once.
const cache = new Map();
const MAX_CACHED = 200;

const isLocal = (src) => /^(blob:|data:)/.test(src);

const load = (src) => {
  if (!cache.has(src)) {
    if (cache.size >= MAX_CACHED) {
      // Oldest first; revoke so the browser can free the image
      const [oldest, pending] = cache.entries().next().value;
      cache.delete(oldest);
      pending.then((url) => URL.revokeObjectURL(url)).catch(() => {});
    }
    const pending = api.fetchFileBlob(src).then((blob) => URL.createObjectURL(blob));
    // A failed download must not be remembered as the answer
    pending.catch(() => { if (cache.get(src) === pending) cache.delete(src); });
    cache.set(src, pending);
  }
  return cache.get(src);
};

/**
 * @param {string|null} src protected API URL of a file, or a local blob:/data: preview
 * @returns {{ url: string|null, failed: boolean }} url is null while loading
 */
export default function useFileUrl(src) {
  const [state, setState] = useState({ src: null, url: null, failed: false });

  useEffect(() => {
    if (!src) {
      setState({ src: null, url: null, failed: false });
      return undefined;
    }
    if (isLocal(src)) {
      setState({ src, url: src, failed: false });
      return undefined;
    }
    let active = true;
    setState({ src, url: null, failed: false });
    load(src)
      .then((url) => { if (active) setState({ src, url, failed: false }); })
      .catch(() => { if (active) setState({ src, url: null, failed: true }); });
    return () => { active = false; };
  }, [src]);

  // Never show the previous file while a new one is loading
  return state.src === src ? { url: state.url, failed: state.failed } : { url: null, failed: false };
}
