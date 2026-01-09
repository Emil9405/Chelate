// src/components/experiments/ExperimentForm.js
import React from 'react';
import { useExperimentForm } from './hooks/useExperimentForm';
import styles from './ExperimentForm.module.css';

// === Подкомпоненты полей ===

const FormField = ({ label, required, children, fullWidth }) => (
  <div className={`${styles.field} ${fullWidth ? styles.fullWidth : ''}`}>
    <label className={styles.label}>
      {label} {required && <span className={styles.required}>*</span>}
    </label>
    {children}
  </div>
);

const TextInput = ({ name, value, onChange, placeholder, required, type = 'text' }) => (
  <input
    type={type}
    name={name}
    value={value}
    onChange={onChange}
    placeholder={placeholder}
    required={required}
    className={styles.input}
  />
);

const SelectInput = ({ name, value, onChange, children }) => (
  <select
    name={name}
    value={value}
    onChange={onChange}
    className={styles.select}
  >
    {children}
  </select>
);

const TextAreaInput = ({ name, value, onChange, rows = 3 }) => (
  <textarea
    name={name}
    value={value}
    onChange={onChange}
    rows={rows}
    className={styles.textarea}
  />
);

// === Основной компонент ===

const ExperimentForm = ({
  rooms = [],
  onSuccess,
  onCancel,
  // Для внешнего управления редактированием
  editingExperiment = null,
  formRef = null,
}) => {
  const {
    formData,
    isEditing,
    loading,
    error,
    handleInputChange,
    handleTimeChange,
    handleRoomChange,
    handleSubmit,
    loadExperiment,
    clearForm,
    resetForm,
  } = useExperimentForm({ rooms, onSuccess, onCancel });

  // Загружаем данные при изменении editingExperiment
  React.useEffect(() => {
    if (editingExperiment) {
      loadExperiment(editingExperiment);
    } else {
      // Сброс формы при создании нового эксперимента (без закрытия)
      clearForm();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingExperiment]);

  // Экспонируем методы через ref
  React.useImperativeHandle(formRef, () => ({
    loadExperiment,
    resetForm,
  }), [loadExperiment, resetForm]);

  const handleFormSubmit = async (e) => {
    const result = await handleSubmit(e);
    if (result.success) {
      alert(result.message);
    } else {
      alert('Error: ' + result.message);
    }
  };

  const isEducational = formData.experiment_type === 'educational';
  const availableRooms = rooms.filter(r => r.status === 'available' || !r.status);

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>
        {isEditing ? 'Edit Experiment' : 'Create New Experiment'}
      </h3>

      {error && (
        <div className={styles.error}>
          <i className="fas fa-exclamation-circle"></i>
          {error}
        </div>
      )}

      <form onSubmit={handleFormSubmit}>
        <div className={styles.grid}>
          {/* Title */}
          <FormField label="Title" required>
            <TextInput
              name="title"
              value={formData.title}
              onChange={handleInputChange}
              required
            />
          </FormField>

          {/* Date */}
          <FormField label="Date" required>
            <TextInput
              type="date"
              name="experiment_date"
              value={formData.experiment_date}
              onChange={handleInputChange}
              required
            />
          </FormField>

          {/* Type */}
          <FormField label="Type">
            <SelectInput
              name="experiment_type"
              value={formData.experiment_type}
              onChange={handleInputChange}
            >
              <option value="research">Research</option>
              <option value="educational">Educational</option>
            </SelectInput>
          </FormField>

          {/* Room */}
          <FormField label={<><i className="fas fa-door-open" style={{ marginRight: '5px', color: '#8b5cf6' }}></i>Room</>}>
            <SelectInput
              name="room_id"
              value={formData.room_id}
              onChange={(e) => handleRoomChange(e.target.value)}
            >
              <option value="">Select Room</option>
              {availableRooms.map(room => (
                <option key={room.id} value={room.id}>
                  {room.name} {room.capacity ? `(${room.capacity} seats)` : ''}
                </option>
              ))}
            </SelectInput>
          </FormField>

          {/* Start Time */}
          <FormField label={<>Start Time {isEducational && <span className={styles.required}>*</span>}</>}>
            <TextInput
              name="start_time"
              value={formData.start_time}
              onChange={handleTimeChange}
              placeholder="HH:MM"
              required={isEducational}
            />
          </FormField>

          {/* End Time */}
          <FormField label={<>End Time {isEducational && <span className={styles.required}>*</span>}</>}>
            <TextInput
              name="end_time"
              value={formData.end_time}
              onChange={handleTimeChange}
              placeholder="HH:MM"
              required={isEducational}
            />
          </FormField>

          {/* Instructor */}
          <FormField label="Instructor">
            <TextInput
              name="instructor"
              value={formData.instructor}
              onChange={handleInputChange}
            />
          </FormField>

          {/* Student Group */}
          <FormField label="Student Group">
            <TextInput
              name="student_group"
              value={formData.student_group}
              onChange={handleInputChange}
            />
          </FormField>

          {/* Description */}
          <FormField label="Description" fullWidth>
            <TextAreaInput
              name="description"
              value={formData.description}
              onChange={handleInputChange}
            />
          </FormField>
        </div>

        {/* Buttons */}
        <div className={styles.buttons}>
          <button
            type="submit"
            disabled={loading}
            className={`${styles.button} ${styles.submitButton}`}
          >
            {loading ? (
              <><i className="fas fa-spinner fa-spin"></i> Saving...</>
            ) : (
              isEditing ? 'Update' : 'Create'
            )}
          </button>
          <button
            type="button"
            onClick={resetForm}
            disabled={loading}
            className={`${styles.button} ${styles.cancelButton}`}
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
};

export default ExperimentForm;
