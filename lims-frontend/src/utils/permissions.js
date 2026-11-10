// utils/permissions.js
// One answer to "may this user do X" for the whole UI.
//
// `user.permissions` comes from /auth/profile and is the exact set the server
// enforces (role defaults or the user's custom permissions; admins have
// everything). Components must ask `can(user, key)` and not look at the role:
// a role check hides buttons from a viewer who was granted the right, and
// shows them to a researcher who had it revoked.

export const isAdmin = (user) => String(user?.role || '').toLowerCase() === 'admin';

export const can = (user, key) => {
  if (!user) return false;
  if (isAdmin(user)) return true;
  const perms = user.permissions;
  if (Array.isArray(perms)) return perms.includes(key);
  if (perms && typeof perms === 'object') return perms[key] === true;
  return false;
};

export const canAny = (user, keys) => keys.some((key) => can(user, key));
