// components/labels/codes.js
// Which kind of Chelate label a scanned or typed code is.
//
// Bottle labels:   CHL:C:<hex>   (typed: 8 hex digits)
// Location labels: CHL:L:<R|Z|P>:<hex>   (typed: R, Z or P + 8 hex digits)
// The server does the real parsing; this only routes a code before it is sent.

export const looksLikeLocationCode = (raw) => {
  const s = String(raw || '').trim();
  if (/^CHL:L:/i.test(s)) return true;
  return /^[RZP][\s:\-·]?[0-9a-f](?:[\s-]?[0-9a-f]){5,11}$/i.test(s);
};

export const KIND_LABEL = { room: 'Room', zone: 'Cabinet', position: 'Shelf' };
export const KIND_COLOR = { room: '#6b46c1', zone: '#2b6cb0', position: '#2f855a' };
