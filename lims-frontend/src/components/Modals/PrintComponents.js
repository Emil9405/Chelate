// components/modals/PrintComponents.js - Label Printing Components

import React, { useState, useEffect } from 'react';
import Modal from '../Modal';
import Button from '../Button';
import { CheckIcon, CloseIcon, FlaskIcon } from '../Icons';
import { labelStyles } from './styles';
import { getExpiryStatus } from './helpers';

// Local PrinterIcon
const PrinterIcon = ({ size = 24, color = "currentColor" }) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="6 9 6 2 18 2 18 9"></polyline>
    <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path>
    <rect x="6" y="14" width="12" height="8"></rect>
  </svg>
);

/**
 * Reagent Label Component
 * Renders a printable sticker for a reagent batch
 */
export const ReagentLabel = ({ reagent, batch }) => {
  const ghsCodes = reagent.hazard_pictograms 
    ? reagent.hazard_pictograms.split(',').filter(Boolean) 
    : [];
  const dateStr = batch?.received_date 
    ? new Date(batch.received_date).toLocaleDateString('ru-RU')
    : new Date().toLocaleDateString('ru-RU');
  const batchId = batch?.batch_number || batch?.id || 'N/A';
  const hasHazards = ghsCodes.length > 0;

  return (
    <div className="printable-sticker" style={labelStyles.container}>
      <div style={labelStyles.header}>
        <h1 style={labelStyles.title}>{reagent.name}</h1>
        <div style={labelStyles.subHeader}>
          CAS: {reagent.cas_number || '—'} | MW: {reagent.molecular_weight || '—'} g/mol
        </div>
      </div>

      <div style={labelStyles.body}>
        <div style={labelStyles.leftCol}>
          <div>
            <div style={{ fontSize: '10px', color: '#555', marginBottom: '2px' }}>
              Formula:
            </div>
            <div style={labelStyles.formulaBox}>{reagent.formula || '—'}</div>
          </div>
          <div style={labelStyles.storageBox}>
            <div style={{ fontSize: '9px', textTransform: 'uppercase', marginBottom: '1px' }}>
              Storage:
            </div>
            {reagent.storage_conditions || 'Ambient'}
          </div>
        </div>

        <div style={labelStyles.rightCol}>
          {hasHazards ? (
            <div style={labelStyles.ghsGrid}>
              {ghsCodes.slice(0, 4).map(code => (
                <div key={code} style={labelStyles.ghsDiamond}>
                  <img 
                    src={`/assets/ghs/${code}.svg`} 
                    alt={code} 
                    style={labelStyles.ghsIcon}
                    onError={(e) => { e.target.style.display = 'none'; }} 
                  />
                </div>
              ))}
            </div>
          ) : (
            <div style={{
              ...labelStyles.storageBox, 
              borderColor: '#38a169', 
              color: '#2f855a', 
              textAlign: 'center', 
              height: 'auto',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center'
            }}>
              No Hazards
            </div>
          )}
        </div>
      </div>

      <div style={labelStyles.footer}>
        <span>{dateStr}</span>
        <span>ID: {batchId}</span>
      </div>

      <div style={{
        ...labelStyles.stripe,
        background: hasHazards 
          ? 'repeating-linear-gradient(-45deg, #f6e05e, #f6e05e 10px, #1a202c 10px, #1a202c 20px)' 
          : '#38a169'
      }} />
    </div>
  );
};

/**
 * Generate printable HTML for labels
 */
const generateLabelHTML = (reagent, batch) => {
  const ghsCodes = reagent.hazard_pictograms 
    ? reagent.hazard_pictograms.split(',').filter(Boolean) 
    : [];
  const hasHazards = ghsCodes.length > 0;
  const dateStr = batch?.received_date 
    ? new Date(batch.received_date).toLocaleDateString('ru-RU')
    : new Date().toLocaleDateString('ru-RU');

  return `
    <div class="printable-sticker" style="
      width: 380px; height: 220px; border: 2px solid #000; border-radius: 8px;
      padding: 12px; font-family: Arial, sans-serif; background: white;
      position: relative; display: flex; flex-direction: column;
      justify-content: space-between; overflow: hidden; box-sizing: border-box;
    ">
      <div style="border-bottom: 2px solid #000; padding-bottom: 6px; margin-bottom: 6px;">
        <h1 style="font-size: 26px; font-weight: 900; margin: 0; line-height: 1;
          text-transform: uppercase; color: #000; white-space: nowrap;
          overflow: hidden; text-overflow: ellipsis;">
          ${reagent.name}
        </h1>
        <div style="font-size: 11px; font-weight: 600; color: #000; margin-top: 4px;">
          CAS: ${reagent.cas_number || '—'} | MW: ${reagent.molecular_weight || '—'} g/mol
        </div>
      </div>
      <div style="display: flex; justify-content: space-between; flex: 1; padding-top: 4px; gap: 10px;">
        <div style="display: flex; flex-direction: column; gap: 10px; flex: 1;">
          <div>
            <div style="font-size: 10px; color: #555; margin-bottom: 2px;">Formula:</div>
            <div style="font-size: 28px; font-weight: bold; font-family: monospace; line-height: 1;">
              ${reagent.formula || '—'}
            </div>
          </div>
          <div style="border: 2px solid #000; padding: 4px 6px; font-size: 11px; font-weight: bold; max-width: 120px;">
            <div style="font-size: 9px; text-transform: uppercase; margin-bottom: 1px;">Storage:</div>
            ${reagent.storage_conditions || 'Ambient'}
          </div>
        </div>
        <div style="display: flex; flex-direction: column; align-items: flex-end; justify-content: flex-start;">
          ${hasHazards ? `
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 2px;">
              ${ghsCodes.slice(0, 4).map(code => `
                <div style="width: 50px; height: 50px; border: 2px solid #e53e3e;
                  transform: rotate(45deg); display: flex; align-items: center;
                  justify-content: center; margin: 10px; background: #fff;">
                  <img src="/assets/ghs/${code}.svg" alt="${code}"
                    style="width: 60px; height: 60px; transform: rotate(-45deg); object-fit: contain;"
                    onerror="this.style.display='none';" />
                </div>
              `).join('')}
            </div>
          ` : `
            <div style="border: 2px solid #38a169; padding: 4px 6px; font-size: 11px;
              font-weight: bold; color: #2f855a; text-align: center;">
              No Hazards
            </div>
          `}
        </div>
      </div>
      <div style="font-size: 10px; font-weight: bold; margin-top: auto; margin-bottom: 14px;
        display: flex; justify-content: space-between; z-index: 2;">
        <span>${dateStr}</span>
        <span>ID: ${batch.batch_number || batch.id}</span>
      </div>
      <div style="position: absolute; bottom: 0; left: 0; right: 0; height: 14px;
        background: ${hasHazards 
          ? 'repeating-linear-gradient(-45deg, #f6e05e, #f6e05e 10px, #1a202c 10px, #1a202c 20px)' 
          : '#38a169'};
        border-top: 2px solid #000;"></div>
    </div>
  `;
};

/**
 * Print Sticker Modal Component
 * Enhanced modal for printing multiple stickers with batch selection
 */
export const PrintStickerModal = ({ 
  isOpen, 
  onClose, 
  reagent, 
  batches = [], 
  preSelectedBatchId = null 
}) => {
  const [selectedBatches, setSelectedBatches] = useState(new Set());
  const [copies, setCopies] = useState(1);
  const [previewBatch, setPreviewBatch] = useState(null);

  // Initialize selection
  useEffect(() => {
    if (isOpen) {
      if (preSelectedBatchId) {
        setSelectedBatches(new Set([preSelectedBatchId]));
        const batch = batches.find(b => b.id === preSelectedBatchId);
        setPreviewBatch(batch || batches[0] || null);
      } else if (batches.length > 0) {
        setSelectedBatches(new Set([batches[0].id]));
        setPreviewBatch(batches[0]);
      }
    }
  }, [isOpen, preSelectedBatchId, batches]);

  const toggleBatch = (batchId) => {
    setSelectedBatches(prev => {
      const newSet = new Set(prev);
      if (newSet.has(batchId)) {
        newSet.delete(batchId);
      } else {
        newSet.add(batchId);
      }
      return newSet;
    });
    const batch = batches.find(b => b.id === batchId);
    if (batch) setPreviewBatch(batch);
  };

  const selectAll = () => {
    setSelectedBatches(new Set(batches.map(b => b.id)));
    if (batches.length > 0) setPreviewBatch(batches[0]);
  };

  const deselectAll = () => {
    setSelectedBatches(new Set());
  };

  const handlePrint = () => {
    const selectedBatchList = batches.filter(b => selectedBatches.has(b.id));
    if (selectedBatchList.length === 0) return;

    // Create print content
    const printContent = document.createElement('div');
    printContent.id = 'print-stickers-container';
    printContent.style.cssText = 'position: fixed; top: 0; left: 0; z-index: 99999;';

    // Generate labels for each selected batch × copies
    selectedBatchList.forEach(batch => {
      for (let i = 0; i < copies; i++) {
        const labelWrapper = document.createElement('div');
        labelWrapper.className = 'sticker-page';
        labelWrapper.style.cssText = 'page-break-after: always; padding: 10px;';
        labelWrapper.innerHTML = generateLabelHTML(reagent, batch);
        printContent.appendChild(labelWrapper);
      }
    });

    document.body.appendChild(printContent);

    // Inject print styles
    const printStyle = document.createElement('style');
    printStyle.id = 'print-stickers-style';
    printStyle.innerHTML = `
      @media print {
        body > *:not(#print-stickers-container) { display: none !important; }
        #print-stickers-container { display: block !important; }
        #print-stickers-container .sticker-page {
          page-break-after: always;
          display: flex;
          justify-content: center;
          align-items: flex-start;
          padding: 10mm;
        }
        #print-stickers-container .sticker-page:last-child {
          page-break-after: auto;
        }
        .printable-sticker {
          transform-origin: top left;
        }
      }
      @page {
        size: auto;
        margin: 5mm;
      }
    `;
    document.head.appendChild(printStyle);

    // Print
    window.print();

    // Cleanup
    setTimeout(() => {
      document.body.removeChild(printContent);
      document.head.removeChild(printStyle);
    }, 500);
  };

  if (!isOpen || !reagent) return null;

  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: 'rgba(0,0,0,0.6)', display: 'flex',
      alignItems: 'center', justifyContent: 'center', zIndex: 3000
    }}>
      <div style={{
        backgroundColor: 'white', borderRadius: '16px', padding: '0',
        maxWidth: '900px', width: '95%', maxHeight: '90vh',
        boxShadow: '0 25px 50px rgba(0,0,0,0.25)', overflow: 'hidden',
        display: 'flex', flexDirection: 'column'
      }}>
        {/* Header */}
        <div style={{
          padding: '20px 24px', borderBottom: '1px solid #e2e8f0',
          background: 'linear-gradient(135deg, rgba(49, 130, 206, 0.08) 0%, rgba(56, 161, 105, 0.08) 100%)',
          display: 'flex', justifyContent: 'space-between', alignItems: 'center'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div style={{
              width: '40px', height: '40px', borderRadius: '10px',
              background: 'linear-gradient(135deg, #3182ce, #38a169)',
              display: 'flex', alignItems: 'center', justifyContent: 'center'
            }}>
              <PrinterIcon size={20} color="white" />
            </div>
            <div>
              <h2 style={{ margin: 0, fontSize: '1.25rem', fontWeight: '700', color: '#1a365d' }}>
                Print Stickers
              </h2>
              <p style={{ margin: 0, fontSize: '0.875rem', color: '#718096' }}>
                {reagent.name}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              border: 'none', background: '#f7fafc', width: '36px', height: '36px',
              borderRadius: '8px', cursor: 'pointer', display: 'flex',
              alignItems: 'center', justifyContent: 'center', color: '#718096',
              transition: 'all 0.2s'
            }}
            onMouseEnter={e => { 
              e.target.style.background = '#edf2f7'; 
              e.target.style.color = '#1a365d'; 
            }}
            onMouseLeave={e => { 
              e.target.style.background = '#f7fafc'; 
              e.target.style.color = '#718096'; 
            }}
          >
            <CloseIcon size={20} />
          </button>
        </div>

        {/* Content */}
        <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          {/* Left: Batch Selection */}
          <div style={{
            width: '320px', borderRight: '1px solid #e2e8f0',
            display: 'flex', flexDirection: 'column', backgroundColor: '#f8fafc'
          }}>
            <div style={{
              padding: '16px', borderBottom: '1px solid #e2e8f0',
              display: 'flex', justifyContent: 'space-between', alignItems: 'center'
            }}>
              <span style={{ fontWeight: '600', fontSize: '0.875rem', color: '#1a365d' }}>
                Select Batches ({selectedBatches.size}/{batches.length})
              </span>
              <div style={{ display: 'flex', gap: '8px' }}>
                <button
                  onClick={selectAll}
                  style={{
                    padding: '4px 10px', fontSize: '0.75rem', fontWeight: '600',
                    border: '1px solid #3182ce', background: 'transparent',
                    color: '#3182ce', borderRadius: '6px', cursor: 'pointer'
                  }}
                >
                  All
                </button>
                <button
                  onClick={deselectAll}
                  style={{
                    padding: '4px 10px', fontSize: '0.75rem', fontWeight: '600',
                    border: '1px solid #e2e8f0', background: 'transparent',
                    color: '#718096', borderRadius: '6px', cursor: 'pointer'
                  }}
                >
                  None
                </button>
              </div>
            </div>
            
            <div style={{ flex: 1, overflowY: 'auto', padding: '12px' }}>
              {batches.length === 0 ? (
                <div style={{ textAlign: 'center', padding: '40px 20px', color: '#a0aec0' }}>
                  <FlaskIcon size={32} color="#cbd5e0" />
                  <p style={{ margin: '12px 0 0' }}>No batches available</p>
                </div>
              ) : (
                batches.map(batch => {
                  const isSelected = selectedBatches.has(batch.id);
                  const expiry = getExpiryStatus(batch.expiry_date);
                  return (
                    <div
                      key={batch.id}
                      onClick={() => toggleBatch(batch.id)}
                      style={{
                        padding: '12px 14px', marginBottom: '8px',
                        borderRadius: '10px', cursor: 'pointer',
                        border: isSelected ? '2px solid #3182ce' : '1px solid #e2e8f0',
                        backgroundColor: isSelected ? '#ebf8ff' : 'white',
                        transition: 'all 0.15s ease'
                      }}
                    >
                      <div style={{ 
                        display: 'flex', 
                        justifyContent: 'space-between', 
                        alignItems: 'center' 
                      }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                          <div style={{
                            width: '20px', height: '20px', borderRadius: '4px',
                            border: isSelected ? 'none' : '2px solid #cbd5e0',
                            background: isSelected ? '#3182ce' : 'white',
                            display: 'flex', alignItems: 'center', justifyContent: 'center'
                          }}>
                            {isSelected && <CheckIcon size={14} color="white" />}
                          </div>
                          <span style={{ fontWeight: '600', color: '#1a365d' }}>
                            {batch.batch_number}
                          </span>
                        </div>
                        <span style={{
                          fontSize: '0.75rem', fontWeight: '600',
                          color: expiry.color
                        }}>
                          {expiry.text}
                        </span>
                      </div>
                      <div style={{
                        marginTop: '6px', paddingLeft: '30px',
                        fontSize: '0.8rem', color: '#718096'
                      }}>
                        {batch.quantity} {batch.unit} • {batch.storage_location || batch.location || 'No location'}
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Right: Preview */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <div style={{
              padding: '16px 20px', borderBottom: '1px solid #e2e8f0',
              display: 'flex', justifyContent: 'space-between', alignItems: 'center'
            }}>
              <span style={{ fontWeight: '600', fontSize: '0.875rem', color: '#1a365d' }}>
                Preview
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <span style={{ fontSize: '0.875rem', color: '#718096' }}>Copies:</span>
                <input
                  type="number"
                  min="1"
                  max="10"
                  value={copies}
                  onChange={(e) => setCopies(Math.max(1, Math.min(10, parseInt(e.target.value) || 1)))}
                  style={{
                    width: '60px', padding: '6px 10px', borderRadius: '8px',
                    border: '1px solid #e2e8f0', fontSize: '0.875rem',
                    textAlign: 'center'
                  }}
                />
              </div>
            </div>
            
            <div style={{
              flex: 1, overflow: 'auto', padding: '24px',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'linear-gradient(135deg, #f7fafc 0%, #edf2f7 100%)'
            }}>
              {previewBatch ? (
                <div style={{
                  padding: '20px', background: 'white', borderRadius: '12px',
                  boxShadow: '0 4px 20px rgba(0,0,0,0.1)'
                }}>
                  <ReagentLabel reagent={reagent} batch={previewBatch} />
                </div>
              ) : (
                <div style={{ textAlign: 'center', color: '#a0aec0' }}>
                  <PrinterIcon size={48} color="#cbd5e0" />
                  <p style={{ marginTop: '12px' }}>Select a batch to preview</p>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div style={{
          padding: '16px 24px', borderTop: '1px solid #e2e8f0',
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          backgroundColor: '#f8fafc'
        }}>
          <div style={{ fontSize: '0.875rem', color: '#718096' }}>
            {selectedBatches.size > 0 ? (
              <span>
                <strong style={{ color: '#1a365d' }}>{selectedBatches.size * copies}</strong> sticker(s) will be printed
              </span>
            ) : (
              <span>Select at least one batch</span>
            )}
          </div>
          <div style={{ display: 'flex', gap: '12px' }}>
            <Button variant="secondary" onClick={onClose} icon={<CloseIcon size={16} />}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handlePrint}
              disabled={selectedBatches.size === 0}
              icon={<PrinterIcon size={16} />}
            >
              Print {selectedBatches.size > 0 ? `(${selectedBatches.size * copies})` : ''}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

/**
 * Legacy single-batch print modal (kept for backwards compatibility)
 */
export const PrintLabelModal = ({ isOpen, onClose, reagent, batch }) => {
  const handlePrint = () => {
    const printStyle = document.createElement("style");
    printStyle.innerHTML = `
      @media print {
        body * { visibility: hidden; }
        .printable-sticker, .printable-sticker * { visibility: visible; }
        .printable-sticker {
          position: fixed;
          left: 0;
          top: 0;
          margin: 0;
          border: none !important;
          transform-origin: top left;
        }
        .modal-overlay { display: none; }
      }
      @page { size: auto; margin: 0mm; }
    `;
    document.head.appendChild(printStyle);
    window.print();
    setTimeout(() => document.head.removeChild(printStyle), 100);
  };

  if (!isOpen) return null;

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Print Label">
      <div style={{ 
        display: 'flex', 
        flexDirection: 'column', 
        alignItems: 'center', 
        gap: '2rem', 
        padding: '1rem' 
      }}>
        <div style={{ 
          padding: '20px', 
          background: '#f7fafc', 
          borderRadius: '12px', 
          border: '1px dashed #cbd5e0',
          display: 'flex',
          justifyContent: 'center'
        }}>
          <ReagentLabel reagent={reagent} batch={batch} />
        </div>

        <div style={{ 
          textAlign: 'center', 
          maxWidth: '400px', 
          fontSize: '0.9rem', 
          color: '#718096' 
        }}>
          <p>Confirm the details above. Ensure your printer is set to the correct label size.</p>
        </div>

        <div style={{ 
          display: 'flex', 
          gap: '1rem', 
          width: '100%', 
          justifyContent: 'flex-end', 
          borderTop: '1px solid #e2e8f0', 
          paddingTop: '1rem' 
        }}>
          <Button variant="secondary" onClick={onClose} icon={<CloseIcon size={16} />}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handlePrint} icon={<PrinterIcon size={16} />}>
            Print Sticker
          </Button>
        </div>
      </div>
    </Modal>
  );
};

export { PrinterIcon };

export default {
  ReagentLabel,
  PrintStickerModal,
  PrintLabelModal,
  PrinterIcon
};
