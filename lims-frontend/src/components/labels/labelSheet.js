// components/labels/labelSheet.js
// A4 sheets of labels (bottle Data Matrix, location QR): layouts and the printable HTML document.
//
// All sizes are in millimetres. The layouts match common A4 sticker sheets,
// so the same print also fits pre-cut sticker sheets, and each one keeps
// printable margins for plain paper that is cut by hand.

import { dataMatrixSvg, qrSvg } from './barcode';

export const LAYOUTS = {
  small: {
    id: 'small',
    name: 'Small · 38×21 mm',
    hint: '65 per sheet — vials, small bottles',
    cols: 5, rows: 13, w: 38.1, h: 21.2,
    left: 4.75, top: 10.7, gapX: 2.5, gapY: 0,
    pad: 1.2, code: 13, nameSize: 6.5, textSize: 5.4, monoSize: 5.2, nameLines: 2, ghs: 0,
  },
  medium: {
    id: 'medium',
    name: 'Medium · 48.5×25.4 mm',
    hint: '44 per sheet — 100 mL – 1 L bottles',
    cols: 4, rows: 11, w: 48.5, h: 25.4,
    left: 8, top: 8.8, gapX: 0, gapY: 0,
    pad: 1.5, code: 16.5, nameSize: 7.5, textSize: 6, monoSize: 5.6, nameLines: 2, ghs: 0,
  },
  large: {
    id: 'large',
    name: 'Large · 63.5×38.1 mm',
    hint: '21 per sheet — large bottles, with GHS pictograms',
    cols: 3, rows: 7, w: 63.5, h: 38.1,
    left: 7.2, top: 15.15, gapX: 2.54, gapY: 0,
    pad: 2, code: 21, nameSize: 9, textSize: 7, monoSize: 6.5, nameLines: 2, ghs: 4,
  },
};

// Storage location labels: bigger, readable from a distance. 99.1x38.1 and
// 99.1x67.7 mm are the common A4 sticker cells (L7163 / L7165 style).
export const LOCATION_LAYOUTS = {
  shelf: {
    id: 'shelf',
    name: 'Shelf · 99.1×38.1 mm',
    hint: '14 per sheet — shelves, drawers',
    cols: 2, rows: 7, w: 99.1, h: 38.1,
    left: 4.65, top: 15.15, gapX: 2.54, gapY: 0,
    pad: 2.5, code: 30, nameSize: 15, textSize: 8, monoSize: 8.5, nameLines: 2, ghs: 0,
  },
  door: {
    id: 'door',
    name: 'Door · 99.1×67.7 mm',
    hint: '8 per sheet — cabinet doors, refrigerators, rooms',
    cols: 2, rows: 4, w: 99.1, h: 67.7,
    left: 4.65, top: 13.1, gapX: 2.54, gapY: 0,
    pad: 3.5, code: 46, nameSize: 22, textSize: 10, monoSize: 10, nameLines: 3, ghs: 0,
  },
  compact: {
    id: 'compact',
    name: 'Compact · 63.5×38.1 mm',
    hint: '21 per sheet — small shelves and drawers',
    cols: 3, rows: 7, w: 63.5, h: 38.1,
    left: 7.2, top: 15.15, gapX: 2.54, gapY: 0,
    pad: 2, code: 26, nameSize: 11, textSize: 6.5, monoSize: 7, nameLines: 2, ghs: 0,
  },
};

export const perSheet = (layout) => layout.cols * layout.rows;

const escapeHtml = (value) => String(value ?? '')
  .replace(/&/g, '&amp;')
  .replace(/</g, '&lt;')
  .replace(/>/g, '&gt;')
  .replace(/"/g, '&quot;')
  .replace(/'/g, '&#39;');

const fmtDate = (value) => {
  if (!value) return '';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleDateString('ru-RU');
};

const fmtQty = (q) => {
  const n = Number(q);
  if (!Number.isFinite(n)) return '';
  return String(Math.round(n * 1000) / 1000);
};

// GHS codes go into a URL path: only the GHS01..GHS09 form is accepted
const ghsCodes = (value) => (value || '')
  .split(',').map(c => c.trim().toUpperCase()).filter(c => /^GHS0[1-9]$/.test(c));

/** Expands copies and the skipped cells of a partly used sheet into cells */
export const buildCells = (labels, { copies = 1, skip = 0 } = {}) => {
  const cells = Array.from({ length: Math.max(0, skip) }, () => null);
  for (const label of labels) {
    for (let i = 0; i < copies; i++) cells.push(label);
  }
  return cells;
};

const KIND_TITLE = { room: 'ROOM', zone: 'CABINET', position: 'SHELF' };

const isLocationLabel = (cell) => cell && (cell.kind === 'room' || cell.kind === 'zone' || cell.kind === 'position');

// Color of the room, only if it is a plain CSS hex color (it goes into a style attribute)
const safeColor = (c) => (/^#[0-9a-fA-F]{3,8}$/.test(c || '') ? c : '#cbd5e0');

const locationHtml = (l, svg) => {
  // Path without the label's own name: "Lab 101 / Cabinet A" for a shelf
  const parts = String(l.path || '').split(' / ');
  const where = l.kind === 'room' ? '' : parts.slice(0, -1).join(' / ');
  return `<div class="bar" style="background:${safeColor(l.room_color)}"></div>
    <div class="code">${svg}</div>
    <div class="text">
      <div class="kind">${KIND_TITLE[l.kind] || ''}${l.zone_type && l.kind === 'zone' && l.zone_type !== 'cabinet' ? ` · ${escapeHtml(String(l.zone_type).replace(/_/g, ' ').toUpperCase())}` : ''}</div>
      <div class="name">${escapeHtml(l.name)}</div>
      ${where ? `<div class="line">${escapeHtml(where)}</div>` : ''}
      <div class="short">${escapeHtml(l.short_code)}</div>
    </div>`;
};

const labelHtml = (label, svg, layout, assetBase) => {
  if (isLocationLabel(label)) return locationHtml(label, svg);
  const r = label;
  const lines = [];
  if (layout.id !== 'small') {
    const casFormula = [r.cas_number && `CAS ${r.cas_number}`, r.formula].filter(Boolean).join(' · ');
    if (casFormula) lines.push(casFormula);
  }
  lines.push(`${r.batch_number} · #${r.sequence_number}/${r.batch_container_count}`);
  const qty = `${fmtQty(r.original_quantity)} ${r.unit}`;
  const exp = fmtDate(r.expiry_date);
  lines.push(layout.id === 'small' ? (exp ? `Exp ${exp}` : qty) : [qty, exp && `Exp ${exp}`].filter(Boolean).join(' · '));
  if (layout.id === 'large' && r.storage_conditions) lines.push(r.storage_conditions);

  const ghs = layout.ghs > 0 ? ghsCodes(r.hazard_pictograms).slice(0, layout.ghs) : [];
  const ghsHtml = ghs.length
    ? `<div class="ghs">${ghs.map(c => `<img src="${assetBase}/assets/ghs/${c}.svg" alt="${c}">`).join('')}</div>`
    : '';

  return `<div class="code">${svg}</div>
    <div class="text">
      <div class="name">${escapeHtml(r.reagent_name)}</div>
      ${lines.map(l => `<div class="line">${escapeHtml(l)}</div>`).join('')}
      ${ghsHtml}
      <div class="short">${escapeHtml(r.short_code)}</div>
    </div>`;
};

const sheetCss = (layout, cutGuides) => `
  @page { size: A4 portrait; margin: 0; }
  * { box-sizing: border-box; }
  html, body { margin: 0; padding: 0; background: #fff; }
  body { font-family: Arial, Helvetica, sans-serif; color: #000;
    -webkit-print-color-adjust: exact; print-color-adjust: exact; }
  .sheet { position: relative; width: 210mm; height: 297mm; overflow: hidden;
    page-break-after: always; break-after: page; }
  .sheet:last-child { page-break-after: auto; break-after: auto; }
  .cell { position: absolute; width: ${layout.w}mm; height: ${layout.h}mm;
    padding: ${layout.pad}mm; display: flex; align-items: center; gap: ${layout.pad}mm;
    overflow: hidden; ${cutGuides ? 'outline: 0.15mm dashed #b8b8b8; outline-offset: -0.075mm;' : ''} }
  .code { flex: 0 0 ${layout.code}mm; width: ${layout.code}mm; height: ${layout.code}mm; }
  .text { flex: 1; min-width: 0; display: flex; flex-direction: column; justify-content: center;
    gap: 0.3mm; line-height: 1.12; }
  .name { font-weight: 700; font-size: ${layout.nameSize}pt; overflow-wrap: anywhere;
    display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: ${layout.nameLines};
    overflow: hidden; }
  .line { font-size: ${layout.textSize}pt; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .short { font-family: "Courier New", monospace; font-size: ${layout.monoSize}pt; letter-spacing: 0.2mm; }
  .ghs { display: flex; gap: 0.8mm; margin: 0.4mm 0; }
  .ghs img { width: 7mm; height: 7mm; }
  .bar { position: absolute; left: 0; top: 0; bottom: 0; width: 1.8mm; }
  .kind { font-size: ${Math.max(5.5, layout.textSize - 1.5)}pt; letter-spacing: 0.35mm; font-weight: 700; color: #4a5568; }
  @media screen {
    body { background: #e2e8f0; }
    .sheet { margin: 8mm auto; background: #fff; box-shadow: 0 2px 12px rgba(0,0,0,.25); }
  }
`;

/**
 * Full HTML document with one .sheet per A4 page.
 * Data Matrix symbols are generated here (async, wasm).
 */
export const buildSheetDocument = async (cells, layout, {
  cutGuides = true, autoPrint = false, assetBase = '', title = 'Labels',
} = {}) => {
  const per = perSheet(layout);
  const svgCache = new Map();
  for (const cell of cells) {
    if (cell && !svgCache.has(cell.payload)) {
      svgCache.set(cell.payload, isLocationLabel(cell) ? await qrSvg(cell.payload) : await dataMatrixSvg(cell.payload));
    }
  }

  const sheets = [];
  for (let start = 0; start < Math.max(cells.length, 1); start += per) {
    const page = cells.slice(start, start + per);
    const cellsHtml = page.map((cell, i) => {
      if (!cell) return '';
      const col = i % layout.cols;
      const row = Math.floor(i / layout.cols);
      const x = layout.left + col * (layout.w + layout.gapX);
      const y = layout.top + row * (layout.h + layout.gapY);
      return `<div class="cell" style="left:${x.toFixed(2)}mm;top:${y.toFixed(2)}mm">${labelHtml(cell, svgCache.get(cell.payload), layout, assetBase)}</div>`;
    }).join('');
    sheets.push(`<div class="sheet">${cellsHtml}</div>`);
  }

  const printScript = autoPrint
    ? `<script>window.addEventListener('load', function () { setTimeout(function () { window.focus(); window.print(); }, 150); });</script>`
    : '';
  return `<!doctype html><html><head><meta charset="utf-8"><title>${escapeHtml(title)}</title>
    <style>${sheetCss(layout, cutGuides)}</style></head>
    <body>${sheets.join('')}${printScript}</body></html>`;
};
