// components/labels/barcode.js
// Data Matrix encode/decode, loaded on demand.
//
// zxing-wasm (zxing-cpp compiled to WebAssembly) is used for both directions.
// The .wasm files are bundled as static assets, so labels and scanning also
// work when the server has no Internet access (the library's default is to
// fetch them from a CDN).
//
// Decoding prefers the browser's native BarcodeDetector (Chrome on Android)
// and falls back to zxing-wasm elsewhere (iOS Safari, Firefox, desktop).

let writerPromise = null;
let readerPromise = null;

const loadWriter = () => {
  if (!writerPromise) {
    writerPromise = Promise.all([
      import(/* webpackChunkName: "zxing-writer" */ 'zxing-wasm/writer'),
      import('zxing-wasm/writer/zxing_writer.wasm'),
    ]).then(async ([mod, wasm]) => {
      await mod.prepareZXingModule({
        overrides: { locateFile: (path, prefix) => (path.endsWith('.wasm') ? wasm.default : prefix + path) },
        fireImmediately: true,
      });
      return mod;
    }).catch((e) => { writerPromise = null; throw e; });
  }
  return writerPromise;
};

const loadReader = () => {
  if (!readerPromise) {
    readerPromise = Promise.all([
      import(/* webpackChunkName: "zxing-reader" */ 'zxing-wasm/reader'),
      import('zxing-wasm/reader/zxing_reader.wasm'),
    ]).then(async ([mod, wasm]) => {
      await mod.prepareZXingModule({
        overrides: { locateFile: (path, prefix) => (path.endsWith('.wasm') ? wasm.default : prefix + path) },
        fireImmediately: true,
      });
      return mod;
    }).catch((e) => { readerPromise = null; throw e; });
  }
  return readerPromise;
};

/**
 * Square Data Matrix as a self-contained SVG string that scales to its box
 * (viewBox in modules, crisp edges). Includes the 1-module quiet zone.
 */
export const dataMatrixSvg = async (text) => {
  const { writeBarcode } = await loadWriter();
  const result = await writeBarcode(text, { format: 'DataMatrix', options: 'forceSquare' });
  if (result.error) throw new Error(`Data Matrix: ${result.error}`);
  const svg = result.svg || '';
  const w = /<svg[^>]*\swidth="(\d+(?:\.\d+)?)"/.exec(svg);
  const h = /<svg[^>]*\sheight="(\d+(?:\.\d+)?)"/.exec(svg);
  const paths = [...svg.matchAll(/<path[^>]*\sd="([^"]+)"/g)].map(m => m[1]);
  if (!w || !h || paths.length === 0) throw new Error('Data Matrix: unexpected SVG output');
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w[1]} ${h[1]}" shape-rendering="crispEdges" `
    + `preserveAspectRatio="xMidYMid meet" style="display:block;width:100%;height:100%">`
    + `<rect width="${w[1]}" height="${h[1]}" fill="#fff"/>`
    + paths.map(d => `<path d="${d}" fill="#000"/>`).join('')
    + '</svg>';
};

/**
 * Creates a decoder for canvases. Returns { kind, decode(canvas) -> string[] }.
 */
export const createDataMatrixDecoder = async () => {
  if (typeof window !== 'undefined' && 'BarcodeDetector' in window) {
    try {
      const formats = await window.BarcodeDetector.getSupportedFormats();
      if (formats.includes('data_matrix')) {
        const detector = new window.BarcodeDetector({ formats: ['data_matrix'] });
        return {
          kind: 'native',
          decode: async (canvas) => (await detector.detect(canvas)).map(r => r.rawValue).filter(Boolean),
        };
      }
    } catch (_) { /* fall back to zxing-wasm */ }
  }
  const { readBarcodes } = await loadReader();
  return {
    kind: 'zxing',
    decode: async (canvas) => {
      const ctx = canvas.getContext('2d', { willReadFrequently: true });
      const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const results = await readBarcodes(image, {
        formats: ['DataMatrix'], tryHarder: true, maxNumberOfSymbols: 1,
      });
      return results.filter(r => r.isValid).map(r => r.text);
    },
  };
};
