// components/Modals.js
// Backwards-compatible wrapper that re-exports from modular structure
// For new code, import directly from './modals' or specific files

export {
  // Styles
  styles,
  labelStyles,
  
  // Helpers
  useFormSubmit,
  cleanPayload,
  formatDate,
  getExpiryStatus,
  
  // Hazard Components
  GHS_PICTOGRAMS,
  HazardSelect,
  HazardDisplay,
  
  // Print Components
  ReagentLabel,
  PrintStickerModal,
  PrintLabelModal,
  PrinterIcon,
  
  // Change Password
  ChangePasswordModal,
  
  // Equipment Modals
  CreateEquipmentModal,
  EditEquipmentModal,
  
  // Batch Modals
  CreateBatchModal,
  EditBatchModal,
  
  // Reagent Modals
  CreateReagentModal,
  EditReagentModal,
  ViewReagentModal,
  
  // Usage History
  UsageHistoryModal,
  
  // User Modals (placeholders)
  CreateUserModal,
  EditUserModal,
  ViewUserModal,
} from './Modals';

// Default export for compatibility
export { default } from './Modals';
