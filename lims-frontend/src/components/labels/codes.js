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

/** CAS registry number with a valid check digit, e.g. 14371-10-9 */
export const casOk = (raw) => {
  const m = /^(\d{2,7})-(\d{2})-(\d)$/.exec(String(raw || '').trim());
  if (!m) return false;
  const digits = (m[1] + m[2]).split('').reverse();
  const sum = digits.reduce((acc, d, i) => acc + Number(d) * (i + 1), 0);
  return sum % 10 === Number(m[3]);
};
