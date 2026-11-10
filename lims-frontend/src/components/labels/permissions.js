// components/labels/permissions.js
import { can } from '../../utils/permissions';

/** May write off, move and inventory bottles (backend: `edit_batch`) */
export const canActOnStock = (user) => can(user, 'edit_batch');
