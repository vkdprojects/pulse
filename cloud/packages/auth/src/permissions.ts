import { createAccessControl } from 'better-auth/plugins/access';

/**
 * Per-organization access control. Resources the control plane guards, and the actions each
 * role may take. Roles: viewer (read-only), admin (operate the fleet), owner (also manage the
 * org and its members). See docs/admin-panel-design.md §5.
 */
export const statement = {
  server: ['read', 'manage'],
  metrics: ['read'],
  webhook: ['read', 'manage'],
  member: ['read', 'manage'],
  organization: ['read', 'manage'],
} as const;

export const ac = createAccessControl(statement);

export const viewer = ac.newRole({
  server: ['read'],
  metrics: ['read'],
  webhook: ['read'],
  member: ['read'],
  organization: ['read'],
});

export const admin = ac.newRole({
  server: ['read', 'manage'],
  metrics: ['read'],
  webhook: ['read', 'manage'],
  member: ['read'],
  organization: ['read'],
});

export const owner = ac.newRole({
  server: ['read', 'manage'],
  metrics: ['read'],
  webhook: ['read', 'manage'],
  member: ['read', 'manage'],
  organization: ['read', 'manage'],
});

export const roles = { owner, admin, viewer };

export type OrgRole = keyof typeof roles;
