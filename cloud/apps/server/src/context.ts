import type { Auth } from '@vixpi/auth';
import { type Db, member, organization } from '@vixpi/db';
import { and, eq } from 'drizzle-orm';
import { createMasterClient, type MasterClient } from './master';

export interface SessionUser {
  id: string;
  email: string;
  name: string;
}

export interface AuthContext {
  user: SessionUser;
  activeOrgId: string | null;
}

/** Resolve the better-auth session from request headers (cookie). Null if unauthenticated. */
export async function resolveAuth(auth: Auth, headers: Headers): Promise<AuthContext | null> {
  const session = await auth.api.getSession({ headers });
  if (!session) return null;
  return {
    user: { id: session.user.id, email: session.user.email, name: session.user.name },
    activeOrgId: session.session.activeOrganizationId ?? null,
  };
}

export interface OrgContext {
  id: string;
  name: string;
  slug: string;
  /** The caller's role in this org (owner/admin/viewer), or null if not a member. */
  role: string | null;
  /** Whether a pulse-master has been provisioned for this org yet. */
  hasMaster: boolean;
}

/** Load the active org and the caller's role in it, scoped so a user only sees their own org. */
export async function resolveOrg(
  db: Db,
  userId: string,
  orgId: string,
): Promise<OrgContext | null> {
  const [org] = await db.select().from(organization).where(eq(organization.id, orgId)).limit(1);
  if (!org) return null;

  const [mine] = await db
    .select()
    .from(member)
    .where(and(eq(member.organizationId, orgId), eq(member.userId, userId)))
    .limit(1);

  return {
    id: org.id,
    name: org.name,
    slug: org.slug,
    role: mine?.role ?? null,
    hasMaster: Boolean(org.masterUrl && org.masterAdminToken),
  };
}

/** Build a pulse-master client for an org, or null if it has not been provisioned yet. */
export function masterForOrg(org: {
  masterUrl: string | null;
  masterAdminToken: string | null;
}): MasterClient | null {
  if (!org.masterUrl || !org.masterAdminToken) return null;
  return createMasterClient({ baseUrl: org.masterUrl, adminToken: org.masterAdminToken });
}
