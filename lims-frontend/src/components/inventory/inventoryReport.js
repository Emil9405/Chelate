// components/inventory/inventoryReport.js
// Printable record of a finished inventory (browser print -> paper or PDF).

const escapeHtml = (value) => String(value ?? '')
  .replace(/&/g, '&amp;')
  .replace(/</g, '&lt;')
  .replace(/>/g, '&gt;')
  .replace(/"/g, '&quot;')
  .replace(/'/g, '&#39;');

const fmtWhen = (v) => {
  if (!v) return '—';
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString('ru-RU', {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit',
  });
};

const bottle = (i) => i.container_id
  ? `${i.reagent_name || '?'} · ${i.batch_number || '?'} #${i.sequence_number ?? '?'}`
  : `Unknown code: ${i.raw_code || ''}`;

const shortCode = (i) => (i.container_id ? i.container_id.replace(/-/g, '').slice(0, 8).toUpperCase() : '');

const RESOLUTION = {
  moved_here: 'moved to the scanned shelf',
  placed_here: 'placed on the scanned shelf',
  unplaced: 'marked as not placed',
};

const section = (title, rows, columns) => {
  if (rows.length === 0) return '';
  const head = columns.map(c => `<th>${escapeHtml(c.title)}</th>`).join('');
  const body = rows.map(r => `<tr>${columns.map(c => `<td>${escapeHtml(c.value(r))}</td>`).join('')}</tr>`).join('');
  return `<h2>${escapeHtml(title)} (${rows.length})</h2><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
};

export const buildInventoryReport = (detail) => {
  const s = detail.session;
  const c = detail.counts;
  const items = detail.items || [];
  const by = (r) => items.filter(i => i.result === r);
  const corrected = items.filter(i => i.resolution === 'moved_here' || i.resolution === 'placed_here');

  const colBottle = { title: 'Bottle', value: bottle };
  const colCode = { title: 'Code', value: shortCode };
  const colSeen = { title: 'Scanned at', value: i => i.location || '—' };
  const colRecorded = { title: 'Recorded at', value: i => i.expected_location || 'not placed' };
  const colWhen = { title: 'When', value: i => fmtWhen(i.scanned_at) };
  const colBy = { title: 'By', value: i => i.scanned_by_name || '' };
  const colRes = { title: 'Action', value: i => RESOLUTION[i.resolution] || '' };

  const status = s.status === 'closed' ? 'Finished' : s.status === 'cancelled' ? 'Cancelled' : 'In progress';
  return `<!doctype html><html><head><meta charset="utf-8">
  <title>Inventory check ${escapeHtml(s.scope_name)}</title>
  <style>
    @page { size: A4; margin: 14mm; }
    body { font-family: Arial, Helvetica, sans-serif; color: #000; font-size: 10pt; }
    h1 { font-size: 16pt; margin: 0 0 4px; }
    h2 { font-size: 11.5pt; margin: 16px 0 6px; }
    .meta { color: #333; margin-bottom: 10px; line-height: 1.5; }
    .sum { display: flex; flex-wrap: wrap; gap: 6px 18px; padding: 8px 10px; border: 1px solid #999; margin: 8px 0; }
    .sum b { font-size: 12pt; }
    table { width: 100%; border-collapse: collapse; }
    th, td { border: 1px solid #bbb; padding: 3px 5px; text-align: left; vertical-align: top; }
    th { background: #eee; }
    tr { page-break-inside: avoid; }
    .sign { margin-top: 28px; display: flex; gap: 40px; }
    .sign div { flex: 1; border-top: 1px solid #000; padding-top: 3px; font-size: 9pt; }
  </style></head><body>
  <h1>Inventory check — ${escapeHtml(s.scope_name)}</h1>
  <div class="meta">
    Status: <b>${status}</b><br>
    Started: ${fmtWhen(s.started_at)}${s.started_by_name ? ` by ${escapeHtml(s.started_by_name)}` : ''}<br>
    ${s.closed_at ? `Finished: ${fmtWhen(s.closed_at)}${s.closed_by_name ? ` by ${escapeHtml(s.closed_by_name)}` : ''}<br>` : ''}
    ${s.notes ? `Notes: ${escapeHtml(s.notes)}<br>` : ''}
  </div>
  <div class="sum">
    <span>Expected <b>${c.expected}</b></span>
    <span>Found <b>${c.found}</b></span>
    <span>Missing <b>${c.missing}</b></span>
    <span>Misplaced <b>${c.misplaced}</b></span>
    <span>Not placed <b>${c.unplaced}</b></span>
    <span>Empty/disposed <b>${c.inactive}</b></span>
    <span>Unknown codes <b>${c.unknown}</b></span>
  </div>
  ${section('Missing — recorded here, not found', by('missing'), [colBottle, colCode, colRecorded, colRes])}
  ${section('Misplaced — found on another shelf', by('misplaced'), [colBottle, colCode, colSeen, colRecorded, colWhen, colBy])}
  ${section('Not placed in Chelate — found here', by('unplaced'), [colBottle, colCode, colSeen, colWhen, colBy])}
  ${section('Recorded as empty or disposed — found here', by('inactive'), [colBottle, colCode, colSeen, colWhen, colBy])}
  ${section('Unknown codes', by('unknown'), [colBottle, colSeen, colWhen, colBy])}
  ${section('Corrected during the check', corrected, [colBottle, colCode, colRecorded, colSeen, colRes, colBy])}
  ${section('Found in place', by('found').filter(i => !i.resolution), [colBottle, colCode, colSeen, colWhen, colBy])}
  <div class="sign"><div>Performed by</div><div>Checked by</div></div>
  <script>window.addEventListener('load', function () { setTimeout(function () { window.focus(); window.print(); }, 150); });</script>
  </body></html>`;
};
