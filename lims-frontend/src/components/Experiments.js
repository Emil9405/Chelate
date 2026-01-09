// src/components/Experiments.js


import React from 'react';
import {
  ExperimentDetailsModal,
  RoomManager,
  ExperimentForm,
  CalendarView,
  useExperiments,
} from './experiment';

// === Компонент фильтров ===
const ExperimentFilters = ({ filters, setFilter, rooms }) => (
  <div style={{
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
    gap: '15px',
    marginBottom: '20px'
  }}>
    <input
      type="text"
      placeholder="Search experiments..."
      value={filters.search}
      onChange={(e) => setFilter('search', e.target.value)}
      style={{
        padding: '10px',
        border: '1px solid #ddd',
        borderRadius: '5px',
      }}
    />
    <select
      value={filters.status}
      onChange={(e) => setFilter('status', e.target.value)}
      style={{
        padding: '10px',
        border: '1px solid #ddd',
        borderRadius: '5px',
      }}
    >
      <option value="">All Statuses</option>
      <option value="planned">Planned</option>
      <option value="in_progress">In Progress</option>
      <option value="completed">Completed</option>
      <option value="cancelled">Cancelled</option>
    </select>
    <select
      value={filters.room}
      onChange={(e) => setFilter('room', e.target.value)}
      style={{
        padding: '10px',
        border: '1px solid #ddd',
        borderRadius: '5px',
      }}
    >
      <option value="">All Rooms</option>
      {rooms.map(room => (
        <option key={room.id} value={room.name}>
          {room.name}
        </option>
      ))}
    </select>
  </div>
);

// === Компонент карточки эксперимента ===
const ExperimentCard = ({ experiment, onView, onEdit, onDelete, canEdit, canDelete, getRoomColor }) => {
  const statusColors = {
    planned: { bg: '#dbeafe', color: '#1e40af' },
    in_progress: { bg: '#fef3c7', color: '#92400e' },
    completed: { bg: '#d1fae5', color: '#065f46' },
    cancelled: { bg: '#fee2e2', color: '#991b1b' },
  };
  const colors = statusColors[experiment.status] || statusColors.planned;

  return (
    <div style={{
      backgroundColor: 'white',
      padding: '15px',
      borderRadius: '8px',
      boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
      borderLeft: `4px solid ${getRoomColor(experiment.location)}`,
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div>
          <h3 style={{ margin: '0 0 8px 0', fontSize: '16px' }}>{experiment.title}</h3>
          <div style={{ fontSize: '13px', color: '#6b7280' }}>
            {new Date(experiment.experiment_date).toLocaleDateString()}
            {experiment.location && ` • ${experiment.location}`}
          </div>
        </div>
        <span style={{
          padding: '4px 10px',
          borderRadius: '12px',
          fontSize: '12px',
          fontWeight: '500',
          backgroundColor: colors.bg,
          color: colors.color,
        }}>
          {experiment.status}
        </span>
      </div>
      
      <div style={{ marginTop: '12px', display: 'flex', gap: '8px' }}>
        <button onClick={() => onView(experiment.id)} style={{
          padding: '6px 12px', backgroundColor: '#667eea', color: 'white',
          border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '12px'
        }}>
          View
        </button>
        {canEdit && (
          <button onClick={() => onEdit(experiment)} style={{
            padding: '6px 12px', backgroundColor: '#f3f4f6', color: '#374151',
            border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '12px'
          }}>
            Edit
          </button>
        )}
        {canDelete && (
          <button onClick={() => {
            if (window.confirm('Delete this experiment?')) {
              onDelete(experiment.id);
            }
          }} style={{
            padding: '6px 12px', backgroundColor: '#fee2e2', color: '#dc2626',
            border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '12px'
          }}>
            Delete
          </button>
        )}
      </div>
    </div>
  );
};

// === Компонент списка ===
const ExperimentList = ({ experiments, onView, onEdit, onDelete, permissions, getRoomColor }) => {
  if (experiments.length === 0) {
    return (
      <div style={{
        padding: '40px',
        textAlign: 'center',
        backgroundColor: 'white',
        borderRadius: '8px',
        color: '#6b7280',
      }}>
        No experiments found
      </div>
    );
  }

  return (
    <div style={{ display: 'grid', gap: '15px' }}>
      {experiments.map(exp => (
        <ExperimentCard
          key={exp.id}
          experiment={exp}
          onView={onView}
          onEdit={onEdit}
          onDelete={onDelete}
          canEdit={permissions.canEdit}
          canDelete={permissions.canDelete}
          getRoomColor={getRoomColor}
        />
      ))}
    </div>
  );
};

// === Главный компонент ===
const Experiments = ({ user }) => {
  const {
    // Данные
    experiments,
    batches,
    equipment,
    rooms,
    loading,
    
    // Выбранные элементы
    selectedExperiment,
    editingExperiment,
    
    // Фильтры
    filters,
    setFilter,
    
    // UI
    ui,
    setViewMode,
    setCurrentDate,
    
    // Действия
    viewDetails,
    refreshDetails,  // <-- ДОБАВЛЕНО
    closeDetails,
    deleteExperiment,
    
    // Форма
    openCreateForm,
    openEditForm,
    closeForm,
    onFormSuccess,
    
    // Комнаты
    openRoomManager,
    closeRoomManager,
    loadData,
    
    // Утилиты
    getRoomColor,
    permissions,
  } = useExperiments(user);

  if (loading) {
    return <div style={{ padding: '20px', textAlign: 'center' }}>Loading...</div>;
  }

  return (
    <div style={{ padding: '100px 20px 20px', maxWidth: '1400px', margin: '0 auto' }}>
      {/* Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '20px',
        backgroundColor: 'white',
        padding: '20px',
        borderRadius: '8px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        flexWrap: 'wrap',
        gap: '15px',
      }}>
        <h2 style={{ margin: 0, fontSize: '24px', fontWeight: '600' }}>
          <i className="fas fa-flask" style={{ marginRight: '10px', color: '#667eea' }}></i>
          Experiments Management
        </h2>

        <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
          {/* View Mode Switcher */}
          <div style={{ display: 'inline-flex', backgroundColor: '#f3f4f6', borderRadius: '8px', padding: '4px' }}>
            {['list', 'calendar'].map(mode => (
              <button
                key={mode}
                onClick={() => setViewMode(mode)}
                style={{
                  padding: '8px 16px',
                  backgroundColor: ui.viewMode === mode ? '#667eea' : 'transparent',
                  color: ui.viewMode === mode ? 'white' : '#4b5563',
                  border: 'none',
                  borderRadius: '6px',
                  cursor: 'pointer',
                  fontWeight: '500',
                }}
              >
                {mode === 'list' ? 'List' : 'Calendar'}
              </button>
            ))}
          </div>

          {permissions.canCreate && (
            <>
              <button onClick={openRoomManager} style={{
                padding: '10px 20px', backgroundColor: '#8b5cf6', color: 'white',
                border: 'none', borderRadius: '8px', cursor: 'pointer', fontWeight: '600',
              }}>
                <i className="fas fa-door-open"></i> Rooms
              </button>
              <button onClick={openCreateForm} style={{
                padding: '10px 20px', backgroundColor: '#10b981', color: 'white',
                border: 'none', borderRadius: '8px', cursor: 'pointer', fontWeight: '600',
              }}>
                <i className="fas fa-plus"></i> New Experiment
              </button>
            </>
          )}
        </div>
      </div>

      {/* Filters (only in list view) */}
      {ui.viewMode === 'list' && (
        <ExperimentFilters filters={filters} setFilter={setFilter} rooms={rooms} />
      )}

      {/* Form */}
      {ui.showForm && (
        <ExperimentForm
          rooms={rooms}
          editingExperiment={editingExperiment}
          onSuccess={onFormSuccess}
          onCancel={closeForm}
        />
      )}

      {/* Calendar View */}
      {ui.viewMode === 'calendar' && (
        <CalendarView
          experiments={experiments}
          currentDate={ui.currentDate}
          setCurrentDate={setCurrentDate}
          onExperimentClick={viewDetails}
          getRoomColor={getRoomColor}
        />
      )}

      {/* List View */}
      {ui.viewMode === 'list' && (
        <ExperimentList
          experiments={experiments}
          onView={viewDetails}
          onEdit={openEditForm}
          onDelete={deleteExperiment}
          permissions={permissions}
          getRoomColor={getRoomColor}
        />
      )}

      {/* Details Modal */}
      {ui.showDetails && selectedExperiment && (
        <ExperimentDetailsModal
          experimentData={selectedExperiment}
          batches={batches}
          equipment={equipment}
          onClose={closeDetails}
          onUpdate={refreshDetails}  // <-- ИЗМЕНЕНО: было loadData
          canEdit={permissions.canEdit}
          user={user}
        />
      )}

      {/* Room Manager */}
      {ui.showRoomManager && (
        <RoomManager
          rooms={rooms}
          onClose={closeRoomManager}
          onUpdate={loadData}
        />
      )}
    </div>
  );
};

export default Experiments;
