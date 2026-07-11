import type { Db } from '@vixpi/db';
import { betterAuth } from 'better-auth';
import { drizzleAdapter } from 'better-auth/adapters/drizzle';
import { admin, organization } from 'better-auth/plugins';

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
 * and `admin` plugins (owner/admin/viewer roles). Schema lives in `@vixpi/db`.
 * See docs/admin-panel-design.md §5. (Per-org API keys land with the `apiKey` plugin once we
 * move to a better-auth version that ships it.)
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
    plugins: [organization(), admin()],
  });
}

export type Auth = ReturnType<typeof createAuth>;
