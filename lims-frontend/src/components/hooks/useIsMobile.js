// components/hooks/useIsMobile.js
// True when the viewport is phone-sized. Layout containers in this app use
// inline styles (no media queries), so components switch layout in JS.

import { useEffect, useState } from 'react';

export const MOBILE_QUERY = '(max-width: 767px)';

const matches = (query) =>
  typeof window !== 'undefined' && typeof window.matchMedia === 'function'
    ? window.matchMedia(query).matches
    : false;

export default function useIsMobile(query = MOBILE_QUERY) {
  const [isMobile, setIsMobile] = useState(() => matches(query));

  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return undefined;
    const mql = window.matchMedia(query);
    const onChange = (e) => setIsMobile(e.matches);
    setIsMobile(mql.matches);
    if (mql.addEventListener) mql.addEventListener('change', onChange);
    else mql.addListener(onChange); // Safari < 14
    return () => {
      if (mql.removeEventListener) mql.removeEventListener('change', onChange);
      else mql.removeListener(onChange);
    };
  }, [query]);

  return isMobile;
}
