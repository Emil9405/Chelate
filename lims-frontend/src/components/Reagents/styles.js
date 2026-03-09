// components/Reagents/styles.js

export const COLUMN_WIDTHS = {
  expandIcon: '32px',
  actions: '260px',
  gridColumns: '2fr 1fr 80px 1fr 1fr 100px'
};

// Compact batch row: Batch# | Qty | Status | Location | Expiry | ▼
export const BATCH_ROW_COLUMNS = '120px 100px 80px 1fr 100px 28px';

export const accordionStyles = {
  // ==================== Reagent Accordion ====================
  container: {
    border: '1px solid #e2e8f0',
    borderRadius: '12px',
    overflow: 'hidden',
    marginBottom: '8px',
    backgroundColor: '#fff',
    boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)',
    transition: 'all 0.2s ease'
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '14px 18px',
    backgroundColor: '#fff',
    cursor: 'pointer',
    userSelect: 'none',
    transition: 'all 0.2s ease',
    gap: '14px'
  },
  headerHover: {
    backgroundColor: 'rgba(49, 130, 206, 0.04)'
  },
  expandIcon: {
    width: '28px',
    height: '28px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: '#3182ce',
    transition: 'transform 0.2s ease',
    flexShrink: 0,
    backgroundColor: 'rgba(49, 130, 206, 0.1)',
    borderRadius: '8px'
  },
  reagentInfo: {
    flex: 1,
    display: 'grid',
    gridTemplateColumns: COLUMN_WIDTHS.gridColumns,
    gap: '12px',
    alignItems: 'center'
  },
  reagentName: {
    fontWeight: '600',
    color: '#1a365d',
    fontSize: '0.95rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap'
  },
  reagentField: {
    color: '#718096',
    fontSize: '0.875rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap'
  },
  actionsColumn: {
    width: COLUMN_WIDTHS.actions,
    display: 'flex',
    gap: '6px',
    flexShrink: 0,
    justifyContent: 'flex-start'
  },

  // ==================== Batches Section ====================
  batchesContainer: {
    borderTop: '1px solid #e2e8f0',
    backgroundColor: '#f8fafc',
    padding: '18px',
    animation: 'slideDown 0.2s ease-out'
  },
  batchesHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '14px'
  },
  batchesTitle: {
    fontSize: '0.875rem',
    fontWeight: '700',
    color: '#1a365d',
    display: 'flex',
    alignItems: 'center',
    gap: '8px'
  },
  noBatches: {
    textAlign: 'center',
    color: '#a0aec0',
    padding: '24px',
    fontSize: '0.875rem',
    backgroundColor: '#fff',
    borderRadius: '10px',
    border: '1px dashed #e2e8f0'
  },

  // ==================== Compact Batch Row (NEW) ====================
  batchRowHeader: {
    display: 'grid',
    gridTemplateColumns: BATCH_ROW_COLUMNS,
    gap: '10px',
    padding: '8px 14px',
    fontWeight: '700',
    fontSize: '0.7rem',
    color: '#1a365d',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.08) 0%, rgba(56, 161, 105, 0.08) 100%)',
    borderRadius: '8px',
    marginBottom: '4px',
  },
  batchRow: {
    display: 'grid',
    gridTemplateColumns: BATCH_ROW_COLUMNS,
    gap: '10px',
    padding: '10px 14px',
    backgroundColor: '#fff',
    borderTopLeftRadius: '8px',
    borderTopRightRadius: '8px',
    borderBottomLeftRadius: '8px',
    borderBottomRightRadius: '8px',
    borderTopWidth: '1px',
    borderRightWidth: '1px',
    borderBottomWidth: '1px',
    borderLeftWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#e2e8f0',
    marginBottom: '2px',
    alignItems: 'center',
    cursor: 'pointer',
    transition: 'all 0.15s ease',
  },
  batchRowActive: {
    borderColor: '#3182ce',
    borderBottomLeftRadius: '0px',
    borderBottomRightRadius: '0px',
    borderBottomWidth: '0px',
    backgroundColor: '#f7faff',
    marginBottom: '0px',
  },
  batchRowHover: {
    backgroundColor: '#f7fafc',
  },
  batchValue: {
    color: '#1a365d',
    fontWeight: '500',
    fontSize: '0.875rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  batchChevron: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: '#a0aec0',
    transition: 'transform 0.2s ease',
    fontSize: '10px',
  },

  // ==================== Expiry ====================
  expiryWarning: { color: '#ed8936', fontWeight: '600' },
  expiryDanger: { color: '#e53e3e', fontWeight: '600' },
  expiryOk: { color: '#38a169' },

  // ==================== Legacy (unused — can remove later) ====================
  batchCard: {
    display: 'grid',
    gridTemplateColumns: '100px 80px 70px 70px 100px 90px 90px 70px 80px 90px 90px 1fr',
    gap: '12px',
    padding: '14px',
    backgroundColor: '#fff',
    borderRadius: '10px',
    border: '1px solid #e2e8f0',
    marginBottom: '8px',
    alignItems: 'center',
    transition: 'all 0.2s ease'
  },
};
