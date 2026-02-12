// src/components/BatchImportModal.js
import React, { useState, useRef } from 'react';

const BatchImportModal = ({ isOpen, onClose, onImport }) => {
  const [file, setFile] = useState(null);
  const [error, setError] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [successMsg, setSuccessMsg] = useState('');
  const fileInputRef = useRef(null);

  const handleFileChange = (e) => {
    const selectedFile = e.target.files[0];
    if (selectedFile) {
      setFile(selectedFile);
      setError('');
      setSuccessMsg('');
    }
  };

  const handleImport = async () => {
    if (!file) {
      setError('Пожалуйста, выберите файл');
      return;
    }

    setIsProcessing(true);
    setError('');

    try {
      // Вызываем функцию из Reagents.js, которая вызывает API
      const response = await onImport(file);
      
      // Если мы здесь, значит ошибок нет (иначе сработал бы catch)
      // response теперь определен, потому что мы исправили Reagents.js
      const message = response?.message || response?.data?.message || 'Импорт завершен успешно';
      
      setSuccessMsg(`✅ ${message}`);
      
      // Закрываем через 2 секунды
      setTimeout(() => {
        handleClose();
      }, 2000);
    } catch (err) {
      console.error(err);
      setError(err.message || 'Ошибка импорта. Проверьте консоль.');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleClose = () => {
    setFile(null);
    setError('');
    setSuccessMsg('');
    if (fileInputRef.current) fileInputRef.current.value = '';
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div style={styles.overlay}>
      <div style={styles.modal}>
        <div style={styles.header}>
          <h2 style={styles.title}>Импорт данных (Excel)</h2>
          <button onClick={handleClose} style={styles.closeButton}>×</button>
        </div>

        <div style={styles.body}>
          {successMsg ? (
            <div style={styles.successBox}>
              <h3>{successMsg}</h3>
            </div>
          ) : (
            <>
              <div style={styles.instructions}>
                <p>Загрузите Excel-файл (.xlsx) со списком реагентов.</p>
                <p>Файл будет обработан на сервере.</p>
              </div>

              <div style={styles.fileInputSection}>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".xlsx,.xls"
                  onChange={handleFileChange}
                  style={styles.fileInput}
                  disabled={isProcessing}
                />
              </div>

              {error && <div style={styles.errorBox}>{error}</div>}
            </>
          )}
        </div>

        <div style={styles.footer}>
          <button onClick={handleClose} style={styles.cancelButton} disabled={isProcessing}>
            Отмена
          </button>
          {!successMsg && (
            <button 
              onClick={handleImport} 
              style={styles.importButton}
              disabled={isProcessing || !file}
            >
              {isProcessing ? 'Загрузка...' : 'Импортировать'}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

const styles = {
  overlay: { position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 2000 },
  modal: { backgroundColor: 'white', borderRadius: '8px', width: '90%', maxWidth: '500px', padding: '20px', boxShadow: '0 4px 20px rgba(0,0,0,0.2)' },
  header: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px', borderBottom: '1px solid #eee', paddingBottom: '10px' },
  title: { margin: 0, fontSize: '1.25rem' },
  closeButton: { background: 'none', border: 'none', fontSize: '1.5rem', cursor: 'pointer', color: '#666' },
  body: { marginBottom: '20px' },
  instructions: { backgroundColor: '#f8f9fa', padding: '10px', borderRadius: '6px', marginBottom: '15px', fontSize: '0.9rem', color: '#555' },
  fileInput: { width: '100%', padding: '10px', border: '2px dashed #ccc', borderRadius: '6px' },
  errorBox: { marginTop: '15px', padding: '10px', backgroundColor: '#fee2e2', color: '#dc2626', borderRadius: '6px' },
  successBox: { padding: '20px', backgroundColor: '#dcfce7', color: '#16a34a', borderRadius: '6px', textAlign: 'center' },
  footer: { display: 'flex', justifyContent: 'flex-end', gap: '10px' },
  cancelButton: { padding: '8px 16px', border: '1px solid #ddd', backgroundColor: 'white', borderRadius: '4px', cursor: 'pointer' },
  importButton: { padding: '8px 16px', border: 'none', backgroundColor: '#3b82f6', color: 'white', borderRadius: '4px', cursor: 'pointer', opacity: 1 },
};

export default BatchImportModal;