// src/components/experiments/ExperimentDetailsModal.js
import React, { useMemo } from 'react';
import { useExperimentDetails } from './hooks/useExperimentDetails';
import styles from './ExperimentDetailsModal.module.css';

// === Вспомогательные функции ===
const getStatusColor = (status) => {
  const colors = {
    planned: { bg: '#dbeafe', color: '#1e40af', border: '#93c5fd' },
    in_progress: { bg: '#fef3c7', color: '#92400e', border: '#fcd34d' },
    completed: { bg: '#d1fae5', color: '#065f46', border: '#6ee7b7' },
    cancelled: { bg: '#fee2e2', color: '#991b1b', border: '#fca5a5' },
  };
  return colors[status] || colors.planned;
};

const formatDate = (dateStr) => {
  if (!dateStr) return 'N/A';
  const date = new Date(dateStr);
  return date.toLocaleDateString('ru-RU', {
    day: '2-digit',
    month: '2-digit', 
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
};

/**
 * Проверка доступности батча для использования
 * Более гибкая логика - не только 'available', но и другие допустимые статусы
 */
const isBatchUsable = (batch) => {
  // Допустимые статусы (case-insensitive)
  const usableStatuses = ['available', 'active', 'in_stock', 'in stock', 'ok', 'good'];
  const status = (batch.status || '').toLowerCase();
  
  // Проверяем статус ИЛИ если статус пустой/не задан - считаем доступным
  const statusOk = !batch.status || usableStatuses.includes(status);
  
  // Проверяем количество (доступное = quantity - reserved_quantity)
  const availableQty = batch.quantity - (batch.reserved_quantity || 0);
  
  return statusOk && availableQty > 0;
};

/**
 * Получить доступное количество батча
 */
const getAvailableQuantity = (batch) => {
  return batch.quantity - (batch.reserved_quantity || 0);
};

// === Подкомпоненты ===

const ModalHeader = ({ experiment, statusColors, onClose }) => (
  <div className={styles.header}>
    <div>
      <h2 className={styles.title}>
        {experiment?.title || 'Experiment Details'}
      </h2>
      <div className={styles.badges}>
        <span 
          className={styles.statusBadge}
          style={{
            backgroundColor: statusColors.bg,
            color: statusColors.color,
            borderColor: statusColors.border
          }}
        >
          {experiment?.status?.toUpperCase()}
        </span>
        <span className={`${styles.typeBadge} ${experiment?.experiment_type === 'educational' ? styles.educational : styles.research}`}>
          {experiment?.experiment_type === 'educational' ? 'Educational' : 'Research'}
        </span>
      </div>
    </div>
    <button onClick={onClose} className={styles.closeButton}>×</button>
  </div>
);

const StatusActions = ({ experiment, canEdit, loading, onStatusChange }) => {
  if (!canEdit) return null;

  const handleAction = async (status) => {
    const result = await onStatusChange(status);
    if (result.success) {
      alert(result.message);
    } else {
      alert('Failed: ' + result.message);
    }
  };

  const currentStatus = experiment?.status;

  return (
    <div className={styles.statusActions}>
      {currentStatus === 'planned' && (
        <button
          onClick={() => handleAction('in_progress')}
          disabled={loading}
          className={`${styles.actionButton} ${styles.startButton}`}
        >
          <i className="fas fa-play"></i> Start Experiment
        </button>
      )}
      {currentStatus === 'in_progress' && (
        <button
          onClick={() => handleAction('completed')}
          disabled={loading}
          className={`${styles.actionButton} ${styles.completeButton}`}
        >
          <i className="fas fa-check"></i> Complete & Consume Reagents
        </button>
      )}
      {['planned', 'in_progress'].includes(currentStatus) && (
        <button
          onClick={() => handleAction('cancelled')}
          disabled={loading}
          className={`${styles.actionButton} ${styles.cancelButton}`}
        >
          <i className="fas fa-times"></i> Cancel Experiment
        </button>
      )}
    </div>
  );
};

const TabNavigation = ({ activeTab, setActiveTab, reagentsCount, equipmentCount }) => {
  const tabs = ['info', 'reagents', 'equipment', 'documents'];
  
  return (
    <div className={styles.tabs}>
      {tabs.map(tab => (
        <button
          key={tab}
          onClick={() => setActiveTab(tab)}
          className={`${styles.tab} ${activeTab === tab ? styles.activeTab : ''}`}
        >
          {tab.charAt(0).toUpperCase() + tab.slice(1)}
          {tab === 'reagents' && reagentsCount > 0 && ` (${reagentsCount})`}
          {tab === 'equipment' && equipmentCount > 0 && ` (${equipmentCount})`}
        </button>
      ))}
    </div>
  );
};

const InfoTab = ({ experiment }) => (
  <div className={styles.infoGrid}>
    <div>
      <h4 className={styles.sectionTitle}>General Information</h4>
      <div className={styles.infoList}>
        <div><strong>Date:</strong> {formatDate(experiment?.experiment_date)}</div>
        <div><strong>Location:</strong> {experiment?.location || 'Not specified'}</div>
        <div><strong>Instructor:</strong> {experiment?.instructor || 'N/A'}</div>
        <div><strong>Student Group:</strong> {experiment?.student_group || 'N/A'}</div>
      </div>
    </div>
    <div>
      <h4 className={styles.sectionTitle}>Schedule</h4>
      <div className={styles.infoList}>
        <div><strong>Start:</strong> {formatDate(experiment?.start_date)}</div>
        <div><strong>End:</strong> {formatDate(experiment?.end_date)}</div>
        <div><strong>Created:</strong> {formatDate(experiment?.created_at)}</div>
      </div>
    </div>
    <div className={styles.fullWidth}>
      <h4 className={styles.sectionTitle}>Description</h4>
      <p className={styles.description}>
        {experiment?.description || 'No description provided'}
      </p>
    </div>
    {experiment?.protocol && (
      <div className={styles.fullWidth}>
        <h4 className={styles.sectionTitle}>Protocol</h4>
        <p className={styles.description}>{experiment.protocol}</p>
      </div>
    )}
    {experiment?.results && (
      <div className={styles.fullWidth}>
        <h4 className={styles.sectionTitle}>Results</h4>
        <p className={styles.description}>{experiment.results}</p>
      </div>
    )}
  </div>
);

const AddReagentForm = ({ 
  availableBatches,
  filteredBatches,
  selectedBatchId, 
  setSelectedBatchId,
  reagentQuantity,
  setReagentQuantity,
  reagentSearch,
  setReagentSearch,
  notes,
  setNotes,
  onAdd,
  onCancel,
  loading 
}) => {
  // Находим выбранный батч для отображения доступного количества
  const selectedBatch = availableBatches.find(b => b.id === selectedBatchId);
  const maxQuantity = selectedBatch ? getAvailableQuantity(selectedBatch) : 0;

  return (
    <div className={styles.addForm}>
      <h4 className={styles.formTitle}>Add Reagent</h4>
      
      {/* Поиск по названию */}
      <div className={styles.searchField}>
        <label className={styles.label}>
          <i className="fas fa-search" style={{ marginRight: '5px' }}></i>
          Search Reagent
        </label>
        <input
          type="text"
          value={reagentSearch}
          onChange={(e) => setReagentSearch(e.target.value)}
          placeholder="Type reagent name to filter..."
          className={styles.input}
          autoFocus
        />
        {reagentSearch && (
          <small className={styles.searchHint}>
            Found {filteredBatches.length} of {availableBatches.length} batches
          </small>
        )}
      </div>

      <div className={styles.formGrid}>
        <div>
          <label className={styles.label}>Batch *</label>
          <select
            value={selectedBatchId}
            onChange={(e) => {
              setSelectedBatchId(e.target.value);
              setReagentQuantity(''); // Сброс количества при смене батча
            }}
            className={styles.select}
          >
            <option value="">Select batch...</option>
            {filteredBatches.map(b => {
              const availQty = getAvailableQuantity(b);
              return (
                <option key={b.id} value={b.id}>
                  {b.reagent_name || 'Unknown'} — {b.batch_number} 
                  {' '}(avail: {availQty.toFixed(2)} {b.unit})
                </option>
              );
            })}
          </select>
          {filteredBatches.length === 0 && (
            <small className={styles.noResults}>
              {reagentSearch 
                ? 'No batches match your search' 
                : 'No available batches found'}
            </small>
          )}
        </div>
        
        <div>
          <label className={styles.label}>
            Quantity * 
            {selectedBatch && (
              <span className={styles.maxHint}>
                (max: {maxQuantity.toFixed(2)} {selectedBatch.unit})
              </span>
            )}
          </label>
          <input
            type="number"
            step="0.01"
            min="0.01"
            max={maxQuantity || undefined}
            value={reagentQuantity}
            onChange={(e) => setReagentQuantity(e.target.value)}
            className={styles.input}
            placeholder={selectedBatch ? `Up to ${maxQuantity.toFixed(2)}` : 'Select batch first'}
            disabled={!selectedBatchId}
          />
          {reagentQuantity && parseFloat(reagentQuantity) > maxQuantity && (
            <small className={styles.warning}>
              Quantity exceeds available ({maxQuantity.toFixed(2)} {selectedBatch?.unit})
            </small>
          )}
        </div>
        
        <div>
          <label className={styles.label}>Notes</label>
          <input
            type="text"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className={styles.input}
            placeholder="Optional notes..."
          />
        </div>
      </div>
      
      <div className={styles.formActions}>
        <button 
          onClick={onAdd} 
          disabled={loading || !selectedBatchId || !reagentQuantity} 
          className={`${styles.button} ${styles.addButton}`}
        >
          {loading ? <><i className="fas fa-spinner fa-spin"></i> Adding...</> : 'Add'}
        </button>
        <button onClick={onCancel} className={`${styles.button} ${styles.cancelFormButton}`}>
          Cancel
        </button>
      </div>
    </div>
  );
};

const ReagentsTable = ({ reagents, experiment, canEdit, onConsume, onRemove }) => (
  <table className={styles.table}>
    <thead>
      <tr>
        <th>Reagent</th>
        <th>Batch</th>
        <th className={styles.textRight}>Quantity</th>
        <th className={styles.textCenter}>Status</th>
        {canEdit && <th className={styles.textCenter}>Actions</th>}
      </tr>
    </thead>
    <tbody>
      {reagents.map(r => (
        <tr key={r.id}>
          <td>{r.reagent_name}</td>
          <td>{r.batch_number}</td>
          <td className={styles.textRight}>
            {r.quantity_used} {r.unit}
          </td>
          <td className={styles.textCenter}>
            <span className={`${styles.consumedBadge} ${r.is_consumed ? styles.consumed : styles.reserved}`}>
              {r.is_consumed ? 'Consumed' : 'Reserved'}
            </span>
          </td>
          {canEdit && (
            <td className={styles.textCenter}>
              {!r.is_consumed && experiment?.status === 'in_progress' && (
                <button
                  onClick={() => onConsume(r.id)}
                  className={`${styles.tableButton} ${styles.consumeButton}`}
                  title="Mark as consumed"
                >
                  <i className="fas fa-check"></i> Consume
                </button>
              )}
              {!r.is_consumed && ['planned', 'in_progress'].includes(experiment?.status) && (
                <button
                  onClick={() => onRemove(r.id)}
                  className={`${styles.tableButton} ${styles.removeButton}`}
                  title="Remove from experiment"
                >
                  <i className="fas fa-trash"></i> Remove
                </button>
              )}
            </td>
          )}
        </tr>
      ))}
    </tbody>
  </table>
);

const ReagentsTab = ({ 
  experiment, 
  reagents, 
  batches,
  canEdit, 
  hook 
}) => {
  // Фильтруем батчи с более гибкой логикой
  const availableBatches = useMemo(() => {
    return (batches || []).filter(isBatchUsable);
  }, [batches]);

  // Дополнительная фильтрация по поисковому запросу
  const filteredBatches = useMemo(() => {
    if (!hook.reagentSearch.trim()) {
      return availableBatches;
    }
    const search = hook.reagentSearch.toLowerCase().trim();
    return availableBatches.filter(b => 
      (b.reagent_name || '').toLowerCase().includes(search) ||
      (b.batch_number || '').toLowerCase().includes(search) ||
      (b.lot_number || '').toLowerCase().includes(search) ||
      (b.cat_number || '').toLowerCase().includes(search)
    );
  }, [availableBatches, hook.reagentSearch]);

  const canModify = canEdit && !['completed', 'cancelled'].includes(experiment?.status);

  const handleAdd = async () => {
    const result = await hook.handleAddReagent();
    if (result.success) {
      alert(result.message);
    } else {
      alert('Failed: ' + result.message);
    }
  };

  const handleConsume = async (id) => {
    if (!window.confirm('Mark this reagent as consumed? This will deduct from batch quantity.')) return;
    const result = await hook.handleConsumeReagent(id);
    if (result.success) {
      alert(result.message);
    } else {
      alert('Failed: ' + result.message);
    }
  };

  const handleRemove = async (id) => {
    if (!window.confirm('Remove this reagent from experiment? Reserved quantity will be returned to batch.')) return;
    const result = await hook.handleRemoveReagent(id);
    if (result.success) {
      alert(result.message);
    } else {
      alert('Failed: ' + result.message);
    }
  };

  return (
    <div>
      {canModify && (
        <div className={styles.addSection}>
          {!hook.showAddReagent ? (
            <button
              onClick={() => hook.setShowAddReagent(true)}
              className={`${styles.button} ${styles.addButton}`}
            >
              <i className="fas fa-plus"></i> Add Reagent
            </button>
          ) : (
            <AddReagentForm
              availableBatches={availableBatches}
              filteredBatches={filteredBatches}
              selectedBatchId={hook.selectedBatchId}
              setSelectedBatchId={hook.setSelectedBatchId}
              reagentQuantity={hook.reagentQuantity}
              setReagentQuantity={hook.setReagentQuantity}
              reagentSearch={hook.reagentSearch}
              setReagentSearch={hook.setReagentSearch}
              notes={hook.notes}
              setNotes={hook.setNotes}
              onAdd={handleAdd}
              onCancel={hook.resetReagentForm}
              loading={hook.loading}
            />
          )}
        </div>
      )}

      {/* Информация о батчах для отладки */}
      {canModify && !hook.showAddReagent && (
        <div className={styles.batchInfo}>
          <small>
            Available batches: {availableBatches.length} / Total: {batches?.length || 0}
          </small>
        </div>
      )}

      {reagents.length === 0 ? (
        <p className={styles.emptyMessage}>No reagents added to this experiment</p>
      ) : (
        <ReagentsTable 
          reagents={reagents}
          experiment={experiment}
          canEdit={canEdit}
          onConsume={handleConsume}
          onRemove={handleRemove}
        />
      )}
    </div>
  );
};

const AddEquipmentForm = ({ 
  equipment, 
  selectedEquipmentId, 
  setSelectedEquipmentId,
  equipmentQuantity,
  setEquipmentQuantity,
  notes,
  setNotes,
  onAdd,
  onCancel,
  loading 
}) => (
  <div className={styles.addForm}>
    <h4 className={styles.formTitle}>Add Equipment</h4>
    <div className={styles.formGrid}>
      <div>
        <label className={styles.label}>Equipment *</label>
        <select
          value={selectedEquipmentId}
          onChange={(e) => setSelectedEquipmentId(e.target.value)}
          className={styles.select}
        >
          <option value="">Select equipment...</option>
          {(Array.isArray(equipment) ? equipment : []).map(eq => (
            <option key={eq.id} value={eq.id}>
              {eq.name} ({eq.status})
            </option>
          ))}
        </select>
      </div>
      <div>
        <label className={styles.label}>Quantity</label>
        <input
          type="number"
          min="1"
          value={equipmentQuantity}
          onChange={(e) => setEquipmentQuantity(e.target.value)}
          className={styles.input}
        />
      </div>
      <div>
        <label className={styles.label}>Notes</label>
        <input
          type="text"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          className={styles.input}
        />
      </div>
    </div>
    <div className={styles.formActions}>
      <button onClick={onAdd} disabled={loading} className={`${styles.button} ${styles.equipmentAddButton}`}>
        Add
      </button>
      <button onClick={onCancel} className={`${styles.button} ${styles.cancelFormButton}`}>
        Cancel
      </button>
    </div>
  </div>
);

const EquipmentTable = ({ expEquipment, experiment, canEdit, onRemove }) => (
  <table className={styles.table}>
    <thead>
      <tr>
        <th>Equipment</th>
        <th className={styles.textRight}>Quantity</th>
        <th>Notes</th>
        {canEdit && <th className={styles.textCenter}>Actions</th>}
      </tr>
    </thead>
    <tbody>
      {expEquipment.map(eq => (
        <tr key={eq.id}>
          <td>{eq.equipment_name}</td>
          <td className={styles.textRight}>{eq.quantity_used} {eq.unit || 'pcs'}</td>
          <td>{eq.notes || '-'}</td>
          {canEdit && ['planned', 'in_progress'].includes(experiment?.status) && (
            <td className={styles.textCenter}>
              <button
                onClick={() => onRemove(eq.id)}
                className={`${styles.tableButton} ${styles.removeButton}`}
              >
                Remove
              </button>
            </td>
          )}
        </tr>
      ))}
    </tbody>
  </table>
);

const EquipmentTab = ({ 
  experiment, 
  expEquipment, 
  equipment,
  canEdit, 
  hook 
}) => {
  const canModify = canEdit && !['completed', 'cancelled'].includes(experiment?.status);

  const handleAdd = async () => {
    const result = await hook.handleAddEquipment();
    if (result.success) {
      alert(result.message);
    } else {
      alert('Failed: ' + result.message);
    }
  };

  const handleRemove = async (id) => {
    if (!window.confirm('Remove this equipment from experiment?')) return;
    const result = await hook.handleRemoveEquipment(id);
    if (result.success) {
      alert(result.message);
    } else {
      alert('Failed: ' + result.message);
    }
  };

  return (
    <div>
      {canModify && (
        <div className={styles.addSection}>
          {!hook.showAddEquipment ? (
            <button
              onClick={() => hook.setShowAddEquipment(true)}
              className={`${styles.button} ${styles.equipmentAddButton}`}
            >
              + Add Equipment
            </button>
          ) : (
            <AddEquipmentForm
              equipment={equipment}
              selectedEquipmentId={hook.selectedEquipmentId}
              setSelectedEquipmentId={hook.setSelectedEquipmentId}
              equipmentQuantity={hook.equipmentQuantity}
              setEquipmentQuantity={hook.setEquipmentQuantity}
              notes={hook.notes}
              setNotes={hook.setNotes}
              onAdd={handleAdd}
              onCancel={hook.resetEquipmentForm}
              loading={hook.loading}
            />
          )}
        </div>
      )}

      {expEquipment.length === 0 ? (
        <p className={styles.emptyMessage}>No equipment added to this experiment</p>
      ) : (
        <EquipmentTable 
          expEquipment={expEquipment}
          experiment={experiment}
          canEdit={canEdit}
          onRemove={handleRemove}
        />
      )}
    </div>
  );
};

const DocumentsTab = ({ documents, experimentId }) => (
  <div>
    {documents.length === 0 ? (
      <p className={styles.emptyMessage}>No documents attached to this experiment</p>
    ) : (
      <div className={styles.documentsList}>
        {documents.map(doc => (
          <div key={doc.id} className={styles.documentItem}>
            <div>
              <div className={styles.documentName}>{doc.original_filename}</div>
              <div className={styles.documentDate}>
                Uploaded: {formatDate(doc.created_at)}
              </div>
            </div>
            <div className={styles.documentActions}>
              <a
                href={`/api/v1/experiments/${experimentId}/documents/${doc.id}/view`}
                target="_blank"
                rel="noopener noreferrer"
                className={`${styles.docButton} ${styles.viewButton}`}
              >
                View
              </a>
              <a
                href={`/api/v1/experiments/${experimentId}/documents/${doc.id}/download`}
                className={`${styles.docButton} ${styles.downloadButton}`}
              >
                Download
              </a>
            </div>
          </div>
        ))}
      </div>
    )}
  </div>
);

// === Основной компонент ===

const ExperimentDetailsModal = ({ 
  experimentData, 
  batches = [], 
  equipment = [], 
  onClose, 
  onUpdate, 
  canEdit = false,
  user
}) => {
  const hook = useExperimentDetails(experimentData, onUpdate);
  const { experiment, reagents, expEquipment, documents } = hook;
  const statusColors = getStatusColor(experiment?.status);

  return (
    <div className={styles.overlay}>
      <div className={styles.modal}>
        <ModalHeader 
          experiment={experiment} 
          statusColors={statusColors} 
          onClose={onClose} 
        />

        <StatusActions
          experiment={experiment}
          canEdit={canEdit}
          loading={hook.loading}
          onStatusChange={hook.handleStatusChange}
        />

        <TabNavigation
          activeTab={hook.activeTab}
          setActiveTab={hook.setActiveTab}
          reagentsCount={reagents.length}
          equipmentCount={expEquipment.length}
        />

        <div className={styles.content}>
          {hook.activeTab === 'info' && (
            <InfoTab experiment={experiment} />
          )}

          {hook.activeTab === 'reagents' && (
            <ReagentsTab
              experiment={experiment}
              reagents={reagents}
              batches={batches}
              canEdit={canEdit}
              hook={hook}
            />
          )}

          {hook.activeTab === 'equipment' && (
            <EquipmentTab
              experiment={experiment}
              expEquipment={expEquipment}
              equipment={equipment}
              canEdit={canEdit}
              hook={hook}
            />
          )}

          {hook.activeTab === 'documents' && (
            <DocumentsTab 
              documents={documents} 
              experimentId={experiment?.id} 
            />
          )}
        </div>

        <div className={styles.footer}>
          <button onClick={onClose} className={styles.closeFooterButton}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

export default ExperimentDetailsModal;
