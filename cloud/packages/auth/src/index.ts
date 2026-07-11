import type { Db } from '@vixpi/db';
import { betterAuth } from 'better-auth';
import { drizzleAdapter } from 'better-auth/adapters/drizzle';
import { admin, organization } from 'better-auth/plugins';
import { ac, roles } from './permissions';

export interface AuthConfig {
  db: Db;
  /** better-auth signing secret (BETTER_AUTH_SECRET). */
  secret: string;
  /** Public origin of the API (BETTER_AUTH_URL). */
  baseUrl: string;
  /** Origins allowed to use the auth endpoints (the web app). */
  trustedOrigins: string[];
  /** SaaS: public sign-up creates a new org. Disable for a locked-down deployment. */
  allowSignUp?: boolean;
}

/**
 * The better-auth server instance. Multi-tenant SaaS: email/password + the `organization`
 * (owner/admin/viewer roles via access control) and `admin` plugins. Each org carries its
 * pulse-master routing (`masterUrl` / `masterAdminToken`), set server-side only.
 * Schema lives in `@vixpi/db`. See docs/admin-panel-design.md §5.
 */
export function createAuth(config: AuthConfig) {
  return betterAuth({
    secret: config.secret,
    baseURL: config.baseUrl,
    trustedOrigins: config.trustedOrigins,
    database: drizzleAdapter(config.db, { provider: 'pg' }),
    emailAndPassword: {
      enabled: true,
      disableSignUp: config.allowSignUp === false,
    },
    plugins: [
      organization({
        ac,
        roles,
        creatorRole: 'owner',
        schema: {
          organization: {
            additionalFields: {
              // Per-tenant master routing. `input: false` → never set by the client; assigned
              // server-side during org→master provisioning (design §4/§15).
              masterUrl: { type: 'string', required: false, input: false },
              masterAdminToken: { type: 'string', required: false, input: false },
            },
          },
        },
      }),
      admin(),
    ],
  });
}

export type Auth = ReturnType<typeof createAuth>;

export type { OrgRole } from './permissions';
export { ac, admin as adminRole, owner, roles, viewer } from './permissions';
