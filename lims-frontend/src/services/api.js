// services/api.js
// v7.1: Добавлены методы для JWT ротации и обновлена поддержка серверной пагинации

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8080';
const API_V1_BASE = `${API_BASE_URL}/api/v1`;

// ==================== HELPERS ====================

const getAuthToken = () => {
  return localStorage.getItem('token');
};

const handleAuthError = () => {
  localStorage.removeItem('token');
  localStorage.removeItem('user');
  window.location.href = '/login';
};

const apiCall = async (url, options = {}) => {
  const token = getAuthToken();
  
  const headers = {
    'Content-Type': 'application/json',
    ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    ...options.headers,
  };

  try {
    const response = await fetch(url, { ...options, headers });

    if (response.status === 401) {
      handleAuthError();
      throw new Error('Authentication required');
    }

    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
      if (response.status === 204) return null;
      const text = await response.text();
      return text;
    }

    const data = await response.json();

    if (!response.ok) {
      throw new Error(data.message || data.error || `HTTP error! status: ${response.status}`);
    }

    return data;
  } catch (error) {
    console.error(`API Call Error (${url}):`, error);
    throw error;
  }
};

const apiMultipartCall = async (url, fileOrFormData, method = 'POST') => {
  const token = getAuthToken();
  
  let body;
  if (fileOrFormData instanceof FormData) {
    body = fileOrFormData;
  } else {
    body = new FormData();
    body.append('file', fileOrFormData);
  }

  try {
    const response = await fetch(url, {
      method,
      headers: {
        ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
      },
      body: body,
    });

    if (response.status === 401) {
      handleAuthError();
      throw new Error('Authentication required');
    }

    if (!response.ok) {
      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        const errData = await response.json();
        throw new Error(errData.message || errData.error || `Server error: ${response.status}`);
      } else {
        const text = await response.text();
        throw new Error(`Upload failed: ${response.status} ${text}`);
      }
    }

    return await response.json();
  } catch (error) {
    console.error(`File Upload Error (${url}):`, error);
    throw error;
  }
};

const apiBlobCall = async (url, options = {}) => {
  const token = getAuthToken();
  const headers = {
    ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    ...options.headers,
  };

  try {
    const response = await fetch(url, { ...options, headers });

    if (response.status === 401) {
      handleAuthError();
      throw new Error('Authentication required');
    }

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Download failed: ${response.status} ${text}`);
    }

    return await response.blob();
  } catch (error) {
    console.error('API blob call error:', error);
    throw error;
  }
};

// ==================== API OBJECT ====================

export const api = {
  token: null,

  init: function() {
    this.token = localStorage.getItem('token');
  },

  clearToken: function() {
    this.token = null;
    localStorage.removeItem('token');
    localStorage.removeItem('user');
  },

  // ==================== AUTH ====================

  login: async (usernameOrCredentials, password) => {
    let username, pwd;
    if (typeof usernameOrCredentials === 'object' && usernameOrCredentials !== null) {
      username = usernameOrCredentials.username;
      pwd = usernameOrCredentials.password;
    } else {
      username = usernameOrCredentials;
      pwd = password;
    }

    const response = await apiCall(`${API_BASE_URL}/auth/login`, {
      method: 'POST',
      body: JSON.stringify({ username, password: pwd }),
    });
    
    const data = response.data || response;

    if (data.token) {
      localStorage.setItem('token', data.token);
      api.token = data.token;
      if (data.user) localStorage.setItem('user', JSON.stringify(data.user));
    }
    
    return data.user || data;
  },

  logout: () => {
    api.clearToken();
  },

  getProfile: async () => {
    const response = await apiCall(`${API_V1_BASE}/auth/profile`);
    return { success: true, data: response.data || response };
  },

  changePassword: async (passwordData) => {
    return await apiCall(`${API_V1_BASE}/auth/change-password`, {
      method: 'POST',
      body: JSON.stringify(passwordData),
    });
  },
  
  // JWT Rotation
  getJwtRotationStatus: async () => {
    const response = await apiCall(`${API_V1_BASE}/auth/jwt/status`);
    return response.data || response;
  },

  forceJwtRotation: async () => {
    const response = await apiCall(`${API_V1_BASE}/auth/jwt/rotate`, { method: 'POST' });
    return response.data || response;
  },

  // ==================== USERS ====================

  getUsers: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/auth/users?${queryString}`);
    return response.data || response;
  },

  createUser: async (userData) => {
    return await apiCall(`${API_V1_BASE}/auth/users`, {
      method: 'POST',
      body: JSON.stringify(userData),
    });
  },

  updateUser: async (id, userData) => {
    return await apiCall(`${API_V1_BASE}/auth/users/${id}`, {
      method: 'PUT',
      body: JSON.stringify(userData),
    });
  },

  deleteUser: async (id) => {
    return await apiCall(`${API_V1_BASE}/auth/users/${id}`, {
      method: 'DELETE',
    });
  },

  resetUserPassword: async (id, newPassword) => {
    return await apiCall(`${API_V1_BASE}/auth/users/${id}/reset-password`, {
      method: 'PUT',
      body: JSON.stringify({ new_password: newPassword }),
    });
  },

  // ==================== REAGENTS ====================

  getReagents: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/reagents?${queryString}`);
    return response.data || response;
  },

  getReagent: async (id) => {
    const response = await apiCall(`${API_V1_BASE}/reagents/${id}`);
    return response.data || response;
  },
  
  getReagentById: async (id) => api.getReagent(id),
  getReagentDetails: async (id) => api.getReagent(id),

  createReagent: async (reagentData) => {
    return await apiCall(`${API_V1_BASE}/reagents`, {
      method: 'POST',
      body: JSON.stringify(reagentData),
    });
  },

  updateReagent: async (id, reagentData) => {
    return await apiCall(`${API_V1_BASE}/reagents/${id}`, {
      method: 'PUT',
      body: JSON.stringify(reagentData),
    });
  },

  deleteReagent: async (id) => {
    return await apiCall(`${API_V1_BASE}/reagents/${id}`, {
      method: 'DELETE',
    });
  },

  // Cursor-based пагинация
  getReagentsCursor: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/reagents/cursor?${queryString}`);
    return response.data || response;
  },
 
  importReagents: async (file) => {
    const isJson = file.name.toLowerCase().endsWith('.json');
    const endpoint = isJson 
      ? `${API_V1_BASE}/reagents/import/json`
      : `${API_V1_BASE}/reagents/import/excel`;
      
    const result = await apiMultipartCall(endpoint, file);
    return result.data || result;
  },

  exportReagents: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const blob = await apiBlobCall(`${API_V1_BASE}/reagents/export?${queryString}`);
    api.triggerDownload(blob, `reagents-${new Date().toISOString().split('T')[0]}.csv`);
  },

  // ==================== BATCHES ====================

  getBatches: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/batches?${queryString}`);
    return response.data || response;
  },

  getAllBatches: async (params = {}) => api.getBatches(params),

  getBatch: async (reagentId, batchId) => {
    const response = await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches/${batchId}`);
    return response.data || response;
  },

  getReagentBatches: async (reagentId, params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches?${queryString}`);
    return response.data || response;
  },

  createBatch: async (reagentId, batchData) => {
    return await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches`, {
      method: 'POST',
      body: JSON.stringify(batchData),
    });
  },

  updateBatch: async (reagentId, batchId, batchData) => {
    return await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches/${batchId}`, {
      method: 'PUT',
      body: JSON.stringify(batchData),
    });
  },

  deleteBatch: async (reagentId, batchId) => {
    return await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches/${batchId}`, {
      method: 'DELETE',
    });
  },

  useBatch: async (reagentId, batchId, usageData) => {
    return await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches/${batchId}/use`, {
      method: 'POST',
      body: JSON.stringify(usageData),
    });
  },
  
  useReagent: async (reagentId, batchId, usageData) => api.useBatch(reagentId, batchId, usageData),

  getUsageHistory: async (reagentId, batchId, params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/reagents/${reagentId}/batches/${batchId}/usage?${queryString}`);
    return response.data || response;
  },

  importBatches: async (file) => {
    const isJson = file.name.toLowerCase().endsWith('.json');
    const endpoint = isJson 
      ? `${API_V1_BASE}/batches/import/json`
      : `${API_V1_BASE}/batches/import/excel`;
      
    const result = await apiMultipartCall(endpoint, file);
    return result.data || result;
  },

  exportBatches: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const blob = await apiBlobCall(`${API_V1_BASE}/batches/export?${queryString}`);
    api.triggerDownload(blob, `batches-${new Date().toISOString().split('T')[0]}.json`);
  },

  getLowStockItems: async (threshold = 10) => {
    try {
        const response = await apiCall(`${API_V1_BASE}/batches/low-stock?threshold=${threshold}`);
        return response.data || response || [];
    } catch (e) {
        return [];
    }
  },

  getExpiringItems: async (days = 30) => {
    try {
        const response = await apiCall(`${API_V1_BASE}/batches/expiring?days=${days}`);
        return response.data || response || [];
    } catch (e) {
        return [];
    }
  },

  // ==================== EQUIPMENT ====================

  getEquipment: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/equipment?${queryString}`);
    return response.data || response;
  },

  getEquipmentItem: async (id) => {
    const response = await apiCall(`${API_V1_BASE}/equipment/${id}`);
    return response.data || response;
  },

  createEquipment: async (equipmentData) => {
    return await apiCall(`${API_V1_BASE}/equipment`, {
      method: 'POST',
      body: JSON.stringify(equipmentData),
    });
  },

  updateEquipment: async (id, equipmentData) => {
    return await apiCall(`${API_V1_BASE}/equipment/${id}`, {
      method: 'PUT',
      body: JSON.stringify(equipmentData),
    });
  },

  deleteEquipment: async (id) => {
    return await apiCall(`${API_V1_BASE}/equipment/${id}`, {
      method: 'DELETE',
    });
  },

  importEquipment: async (file) => {
    const isJson = file.name.toLowerCase().endsWith('.json');
    const endpoint = isJson 
      ? `${API_V1_BASE}/equipment/import/json`
      : `${API_V1_BASE}/equipment/import/excel`;
      
    const result = await apiMultipartCall(endpoint, file);
    return result.data || result;
  },

  exportEquipment: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const blob = await apiBlobCall(`${API_V1_BASE}/equipment/export?${queryString}`);
    api.triggerDownload(blob, `equipment-${new Date().toISOString().split('T')[0]}.json`);
  },

  // --- Equipment Parts & Maintenance ---

  getEquipmentParts: async (equipmentId) => {
    const response = await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/parts`);
    return response.data || response;
  },

  createEquipmentPart: async (equipmentId, partData) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/parts`, {
      method: 'POST',
      body: JSON.stringify(partData),
    });
  },

  updateEquipmentPart: async (equipmentId, partId, partData) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/parts/${partId}`, {
      method: 'PUT',
      body: JSON.stringify(partData),
    });
  },

  deleteEquipmentPart: async (equipmentId, partId) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/parts/${partId}`, {
      method: 'DELETE',
    });
  },

  getEquipmentMaintenance: async (equipmentId) => {
    const response = await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/maintenance`);
    return response.data || response;
  },

  createMaintenance: async (equipmentId, data) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/maintenance`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  updateMaintenance: async (equipmentId, maintenanceId, data) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/maintenance/${maintenanceId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  completeMaintenance: async (equipmentId, maintenanceId, data) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/maintenance/${maintenanceId}/complete`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  deleteMaintenance: async (equipmentId, maintenanceId) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/maintenance/${maintenanceId}`, {
      method: 'DELETE',
    });
  },
  
  getUpcomingMaintenance: async (days = 30, limit = 10) => {
    const response = await apiCall(`${API_V1_BASE}/equipment/maintenance/upcoming?days=${days}&limit=${limit}`);
    return response.data || response;
  },

  getOverdueMaintenance: async () => {
    const response = await apiCall(`${API_V1_BASE}/equipment/maintenance/overdue`);
    return response.data || response;
  },

  // --- Equipment Files ---

  getEquipmentFiles: async (equipmentId) => {
    const response = await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/files`);
    return response.data || response;
  },

  uploadEquipmentFile: async (equipmentId, file, options = {}) => {
    const { file_type = 'other', description = '', part_id = null } = options;
    const formData = new FormData();
    formData.append('file', file);
    formData.append('file_type', file_type);
    if (description) formData.append('description', description);
    if (part_id) formData.append('part_id', part_id);

    const result = await apiMultipartCall(`${API_V1_BASE}/equipment/${equipmentId}/files`, formData);
    return result.data || result;
  },
  
  getPartFiles: async (equipmentId, partId) => {
    const response = await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/parts/${partId}/files`);
    return response.data || response;
  },

  downloadEquipmentFile: async (equipmentId, fileId, filename) => {
    const blob = await apiBlobCall(`${API_V1_BASE}/equipment/${equipmentId}/files/${fileId}/download`);
    api.triggerDownload(blob, filename || 'download');
  },

  deleteEquipmentFile: async (equipmentId, fileId) => {
    return await apiCall(`${API_V1_BASE}/equipment/${equipmentId}/files/${fileId}`, {
      method: 'DELETE',
    });
  },

  // ==================== EXPERIMENTS ====================

  getExperiments: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/experiments?${queryString}`);
    return response.data || response;
  },

  getExperimentDetails: async (id) => {
    const [expResponse, reagentsResponse] = await Promise.all([
      apiCall(`${API_V1_BASE}/experiments/${id}`),
      apiCall(`${API_V1_BASE}/experiments/${id}/reagents`).catch(() => ({ data: [] })),
    ]);
    
    const experiment = expResponse.data || expResponse;
    const reagents = reagentsResponse.data || reagentsResponse || [];
    
    return {
      experiment,
      reagents: Array.isArray(reagents) ? reagents : [],
    };
  },

  createExperiment: async (data) => {
    return await apiCall(`${API_V1_BASE}/experiments`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  updateExperiment: async (id, data) => {
    return await apiCall(`${API_V1_BASE}/experiments/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  },

  deleteExperiment: async (id) => {
    return await apiCall(`${API_V1_BASE}/experiments/${id}`, {
      method: 'DELETE',
    });
  },
  
  getExperimentsByDateRange: async (startDate, endDate, params = {}) => {
    const queryParams = new URLSearchParams({
      ...params,
      date_from: startDate,
      date_to: endDate,
    }).toString();
    const response = await apiCall(`${API_V1_BASE}/experiments?${queryParams}`);
    return response.data || response;
  },

  getExperimentsStats: async () => {
    const response = await apiCall(`${API_V1_BASE}/experiments/stats`);
    return response.data || response;
  },
  
  getExperimentStats: async () => api.getExperimentsStats(),

  addExperimentReagent: async (id, data) => apiCall(`${API_V1_BASE}/experiments/${id}/reagents`, { method: 'POST', body: JSON.stringify(data) }),
  removeExperimentReagent: async (id, rId) => apiCall(`${API_V1_BASE}/experiments/${id}/reagents/${rId}`, { method: 'DELETE' }),
  getExperimentReagents: async (experimentId) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/reagents`);
    return response.data || response;
  },
  
  consumeExperimentReagent: async (experimentId, reagentId) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/reagents/${reagentId}/consume`, {
      method: 'POST',
    });
    return response.data || response;
  },
  
  startExperiment: async (experimentId) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/start`, {
      method: 'POST',
    });
    return response.data || response;
  },

  completeExperiment: async (experimentId) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/complete`, {
      method: 'POST',
    });
    return response.data || response;
  },

  cancelExperiment: async (experimentId) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/cancel`, {
      method: 'POST',
    });
    return response.data || response;
  },

  autoUpdateExperimentStatuses: async () => {
    const response = await apiCall(`${API_V1_BASE}/experiments/auto-update-statuses`, {
      method: 'POST',
    });
    return response.data || response;
  },

  diagnoseExperimentDates: async () => {
    const response = await apiCall(`${API_V1_BASE}/experiments/diagnose-dates`);
    return response.data || response;
  },
  
  addExperimentEquipment: async (experimentId, equipmentData) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/equipment`, {
      method: 'POST',
      body: JSON.stringify(equipmentData),
    });
    return response.data || response;
  },

  removeExperimentEquipment: async (experimentId, equipmentId) => {
    const response = await apiCall(`${API_V1_BASE}/experiments/${experimentId}/equipment/${equipmentId}`, {
      method: 'DELETE',
    });
    return response.data || response;
  },
  
  uploadExperimentDocument: async (id, file) => {
    const result = await apiMultipartCall(`${API_V1_BASE}/experiments/${id}/documents`, file);
    return result.data || result;
  },
  
  uploadExperimentDocuments: async (id, files) => {
    const fd = new FormData();
    for (let i = 0; i < files.length; i++) {
        fd.append('documents', files[i]);
    }
    const result = await apiMultipartCall(`${API_V1_BASE}/experiments/${id}/documents`, fd);
    return result.data || result;
  },
  
  getExperimentDocumentUrl: async (experimentId, documentId) => {
    const token = getAuthToken();
    return `${API_V1_BASE}/experiments/${experimentId}/documents/${documentId}/view?token=${encodeURIComponent(token)}`;
  },

  downloadExperimentDocument: async (expId, docId, filename) => {
    const blob = await apiBlobCall(`${API_V1_BASE}/experiments/${expId}/documents/${docId}/download`);
    api.triggerDownload(blob, filename);
  },
  
  downloadAndSaveExperimentDocument: async (expId, docId, filename) => api.downloadExperimentDocument(expId, docId, filename),

  deleteExperimentDocument: async (expId, docId) => {
    return await apiCall(`${API_V1_BASE}/experiments/${expId}/documents/${docId}`, { method: 'DELETE' });
  },

  // ==================== DASHBOARD & REPORTS ====================

  getDashboardStats: async () => {
    const response = await apiCall(`${API_V1_BASE}/dashboard/stats`);
    return response.data || response;
  },

  getActivityLog: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/activity?${queryString}`);
    return response.data || response;
  },

  getReports: async (params = {}) => {
    const queryString = new URLSearchParams(params).toString();
    const response = await apiCall(`${API_V1_BASE}/reports?${queryString}`);
    return response.data || response;
  },

  generateReport: async (config, params = {}) => {
    let body = config;
    if (typeof config === 'string') {
      const map = { 
        'low-stock': 'low_stock', 
        'lowstock': 'low_stock',
        'expiring': 'expiring_soon',
        'expiring-soon': 'expiring_soon',
        'all-batches': 'all_batches',
        'allbatches': 'all_batches',
        'expired': 'expired'
      };
      body = { preset: map[config] || config, preset_params: params, ...params };
    }
    const response = await apiCall(`${API_V1_BASE}/reports/generate`, {
      method: 'POST',
      body: JSON.stringify(body),
    });
    return response.data || response;
  },
  
  generateReportFallback: async (params = {}) => {
     return api.getAllBatches(params);
  },

  getReportPresets: async () => {
    try {
      const response = await apiCall(`${API_V1_BASE}/reports/presets`);
      return response.data || response;
    } catch (e) { 
        console.warn('getReportPresets error:', e);
        return []; 
    } 
  },

  getReportFields: async () => {
    try {
      const response = await apiCall(`${API_V1_BASE}/reports/fields`);
      return response.data || response;
    } catch (e) { return []; }
  },
  
  getReportColumns: async () => {
    try {
      const response = await apiCall(`${API_V1_BASE}/reports/columns`);
      return response.data || response;
    } catch (e) { return []; }
  },
  
  getReportFieldValues: async (field) => {
    try {
      const response = await apiCall(`${API_V1_BASE}/reports/field-values/${field}`);
      return response.data || response;
    } catch (e) { return []; }
  },
  
  getFieldValues: async (field) => api.getReportFieldValues(field),
  
  exportReportCSV: async (params = {}) => {
    try {
        const blob = await apiBlobCall(`${API_V1_BASE}/reports/export/csv`, { 
            method: 'POST', 
            body: JSON.stringify(params),
            headers: {'Content-Type': 'application/json'}
        });
        api.triggerDownload(blob, `report.csv`);
        return true;
    } catch (e) {
        console.warn('exportReportCSV error:', e);
        throw e;
    }
  },

  // ==================== ROOMS ====================

  getRooms: async () => {
    const response = await apiCall(`${API_V1_BASE}/rooms`);
    return response.data || response;
  },

  getRoom: async (id) => {
    const response = await apiCall(`${API_V1_BASE}/rooms/${id}`);
    return response.data || response;
  },

  getAvailableRooms: async () => {
    const response = await apiCall(`${API_V1_BASE}/rooms/available`);
    return response.data || response;
  },

  createRoom: async (data) => apiCall(`${API_V1_BASE}/rooms`, { method: 'POST', body: JSON.stringify(data) }),
  updateRoom: async (id, data) => apiCall(`${API_V1_BASE}/rooms/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteRoom: async (id) => apiCall(`${API_V1_BASE}/rooms/${id}`, { method: 'DELETE' }),

  // ==================== UTILS ====================
  
  triggerDownload: (blob, filename) => {
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename || 'download';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
  }
};

export default api;