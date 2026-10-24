// components/labels/ArticleBackfill.js
// Links the catalog numbers already stored in batches to their reagents, so a
// bottle of an article that is in Chelate is recognised at its first scan.
// Shows what would happen first; articles found under several reagents, or that
// could not be read, are listed for a person and left alone.

import React, { useState } from 'react';
import { api } from '../../services/api';
import Button from '../Button';

const SERIES = { merck: 'Merck', sigma: 'Sigma-Aldrich', other: '' };
const box = { background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '10px 12px', marginBottom: '10px', fontSize: '0.85rem', color: '#2d3748' };
const head = { fontSize: '0.72rem', fontWeight: 800, letterSpacing: '0.04em', color: '#4a5568', margin: '10px 0 4px' };
const list = { margin: 0, paddingLeft: '18px', overflowWrap: 'anywhere' };
const linkBtn = { background: 'none', border: 'none', color: '#2b6cb0', fontWeight: 700, padding: '6px 0 0', cursor: 'pointer' };

const article = (series, catalog) => `${SERIES[series] ? `${SERIES[series]} ` : ''}${catalog}`;
const plural = (n, word) => `${n} ${word}${n !== 1 ? 's' : ''}`;

const Section = ({ title, items, render, limit = 8 }) => {
  const [all, setAll] = useState(false);
  if (!items.length) return null;
  const shown = all ? items : items.slice(0, limit);
  return (
    <>
      <div style={head}>{title}</div>
      <ul style={list}>{shown.map(render)}</ul>
      {items.length > limit && (
        <button type="button" style={linkBtn} onClick={() => setAll(!all)}>
          {all ? 'Show fewer' : `Show all ${items.length}`}
        </button>
      )}
    </>
  );
};

const ArticleBackfill = () => {
  const [open, setOpen] = useState(false);
  const [report, setReport] = useState(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const load = async () => {
    setOpen(true);
    setBusy(true);
    setError('');
    try {
      setReport(await api.previewArticleBackfill());
    } catch (e) {
      setError(e.message || 'Could not read the batches');
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    setBusy(true);
    setError('');
    try {
      setReport(await api.applyArticleBackfill());
    } catch (e) {
      setError(e.message || 'Could not link the articles');
    } finally {
      setBusy(false);
    }
  };

  if (!open) {
    return (
      <div style={{ marginBottom: '10px' }}>
        <button type="button" style={{ ...linkBtn, padding: 0 }} onClick={load}>
          Recognise articles of batches already in Chelate
        </button>
      </div>
    );
  }

  return (
    <div style={box}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
        <div style={{ fontWeight: 800, color: '#1a365d' }}>Articles of batches already in Chelate</div>
        <button type="button" style={{ ...linkBtn, padding: 0 }} onClick={() => setOpen(false)}>Close</button>
      </div>
      {busy && !report && <div style={{ marginTop: '6px', color: '#718096' }}>Reading the batches…</div>}
      {error && <div role="alert" style={{ marginTop: '6px', color: '#c53030' }}>{error}</div>}

      {report && (
        <>
          <div style={{ marginTop: '6px' }}>
            {report.applied
              ? <><b>{plural(report.new_links.length, 'article')}</b> linked to their reagents. Their bottles are recognised at the first scan.</>
              : <>The catalog numbers in your batches link <b>{plural(report.new_links.length, 'article')}</b> to their reagents.</>}
            {report.already_linked > 0 && ` ${report.already_linked} were linked before.`}
            {report.batches_without_article > 0 && ` ${plural(report.batches_without_article, 'batch')} have no catalog number.`}
          </div>

          {!report.applied && report.new_links.length > 0 && (
            <Button variant="primary" onClick={apply} loading={busy} disabled={busy} style={{ width: '100%', height: '44px', marginTop: '8px' }}>
              Link {plural(report.new_links.length, 'article')}
            </Button>
          )}

          <Section
            title={`NEEDS A PERSON: ${report.conflicts.length}`}
            items={report.conflicts}
            render={(c) => (
              <li key={`${c.series}:${c.catalog}`}>
                <b>{article(c.series, c.catalog)}</b>:{' '}
                {c.reagents.map(r => `${r.reagent_name} (${plural(r.batches, 'batch')})`).join(', ')}
                {c.linked_to && `; already linked to ${c.linked_to.reagent_name}`}.
                {c.reagents.length > 1 ? ' One article is one product: correct the catalog number of the wrong batches.' : ''}
              </li>
            )}
          />
          <Section
            title={`MERCK OR SIGMA? ${report.ambiguous.length}`}
            items={report.ambiguous}
            render={(a) => (
              <li key={`${a.cat_number}:${a.reagent_id}`}>
                {a.cat_number} ({a.reagent_name}, batch {a.batch_number}). Linked at its first scan instead.
              </li>
            )}
          />
          <Section
            title={`NOT READ AS AN ARTICLE: ${report.unrecognised.length}`}
            items={report.unrecognised}
            render={(a) => (
              <li key={`${a.cat_number}:${a.reagent_id}`}>
                {a.cat_number} ({a.reagent_name}, batch {a.batch_number})
              </li>
            )}
          />
          {!report.applied && (
            <Section
              title="WILL BE LINKED"
              items={report.new_links}
              limit={5}
              render={(l) => (
                <li key={`${l.series}:${l.catalog}`}>
                  {article(l.series, l.catalog)} → {l.reagent_name}
                </li>
              )}
            />
          )}
        </>
      )}
    </div>
  );
};

export default ArticleBackfill;
