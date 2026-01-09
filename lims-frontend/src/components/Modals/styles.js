// components/modals/styles.js - Shared styles for modals

export const styles = {
  formGrid: {
    display: 'grid',
    gap: '1rem',
    marginBottom: '1rem'
  },
  twoColGrid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '1rem'
  },
  threeColGrid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr 1fr',
    gap: '1rem'
  },
  fourColGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(4, 1fr)',
    gap: '1rem'
  },
  buttonContainer: {
    display: 'flex',
    gap: '12px',
    justifyContent: 'flex-end',
    marginTop: '1.5rem',
    paddingTop: '1rem',
    borderTop: '1px solid #e2e8f0'
  },
  error: {
    color: '#c53030',
    backgroundColor: 'rgba(229, 62, 62, 0.08)',
    padding: '12px 16px',
    borderRadius: '10px',
    marginBottom: '1rem',
    fontSize: '0.875rem',
    border: '1px solid rgba(229, 62, 62, 0.2)',
    display: 'flex',
    alignItems: 'center',
    gap: '10px'
  },
  success: {
    color: '#2f855a',
    backgroundColor: 'rgba(56, 161, 105, 0.08)',
    padding: '12px 16px',
    borderRadius: '10px',
    marginBottom: '1rem',
    fontSize: '0.875rem',
    border: '1px solid rgba(56, 161, 105, 0.2)',
    display: 'flex',
    alignItems: 'center',
    gap: '10px'
  },
  sectionTitle: {
    fontSize: '0.8rem',
    fontWeight: '700',
    color: '#1a365d',
    marginBottom: '1rem',
    paddingBottom: '0.75rem',
    borderBottom: '2px solid transparent',
    borderImage: 'linear-gradient(90deg, #3182ce, #38b2ac, #38a169) 1',
    textTransform: 'uppercase',
    letterSpacing: '0.08em',
    display: 'flex',
    alignItems: 'center',
    gap: '8px'
  },
  sectionHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '1rem'
  },
  card: {
    backgroundColor: '#fff',
    padding: '1.5rem',
    borderRadius: '12px',
    border: '1px solid #e2e8f0',
    boxShadow: '0 1px 3px rgba(26, 54, 93, 0.06)',
    transition: 'all 0.2s ease'
  }
};

// Label Specific Styles
export const labelStyles = {
  container: {
    width: '380px',
    height: '220px',
    border: '2px solid #000',
    borderRadius: '8px',
    padding: '12px',
    fontFamily: 'Arial, sans-serif',
    backgroundColor: 'white',
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'space-between',
    overflow: 'hidden',
    boxSizing: 'border-box'
  },
  header: {
    borderBottom: '2px solid #000',
    paddingBottom: '6px',
    marginBottom: '6px'
  },
  title: {
    fontSize: '26px',
    fontWeight: '900',
    margin: 0,
    lineHeight: '1',
    textTransform: 'uppercase',
    color: '#000',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis'
  },
  subHeader: {
    fontSize: '11px',
    fontWeight: '600',
    color: '#000',
    marginTop: '4px'
  },
  body: {
    display: 'flex',
    justifyContent: 'space-between',
    flex: 1,
    paddingTop: '4px',
    gap: '10px'
  },
  leftCol: {
    display: 'flex',
    flexDirection: 'column',
    gap: '10px',
    flex: 1
  },
  formulaBox: {
    fontSize: '28px',
    fontWeight: 'bold',
    fontFamily: 'monospace',
    lineHeight: 1
  },
  storageBox: {
    border: '2px solid #000',
    padding: '4px 6px',
    fontSize: '11px',
    fontWeight: 'bold',
    maxWidth: '120px'
  },
  rightCol: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-end',
    justifyContent: 'flex-start'
  },
  footer: {
    fontSize: '10px',
    fontWeight: 'bold',
    marginTop: 'auto',
    marginBottom: '14px',
    display: 'flex',
    justifyContent: 'space-between',
    zIndex: 2
  },
  stripe: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    height: '14px',
    background: 'repeating-linear-gradient(-45deg, #f6e05e, #f6e05e 10px, #1a202c 10px, #1a202c 20px)',
    borderTop: '2px solid #000'
  },
  ghsGrid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '2px'
  },
  ghsDiamond: {
    width: '50px',
    height: '50px',
    border: '2px solid #e53e3e',
    transform: 'rotate(45deg)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    margin: '10px',
    backgroundColor: '#fff'
  },
  ghsIcon: {
    width: '60px',
    height: '60px',
    transform: 'rotate(-45deg)',
    objectFit: 'contain'
  }
};

export default styles;
