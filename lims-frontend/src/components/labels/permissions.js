// components/labels/permissions.js

/** May write off, move and inventory bottles (backend: batch edit permission) */
export const canActOnStock = (user) => {
  const role = (user?.role || '').toString().toLowerCase();
  const perms = user?.permissions;
  const hasPerm = (k) => (Array.isArray(perms) ? perms.includes(k) : !!(perms && perms[k] === true));
  return role === 'admin' || role === 'researcher' || hasPerm('edit_batch');
};
