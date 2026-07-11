import type { Auth } from '@vixpi/auth';
import type { FetchLike } from '@vixpi/contracts';
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

export interface ServerContextError {
  status: number;
  code: string;
  message: string;
}

export interface ServerContext {
  user: SessionUser;
  org: OrgContext;
  master: MasterClient;
  /** The org master's REST base URL (not secret; the admin token is never exposed). */
  masterUrl: string;
}

/**
 * Full guard for org-scoped, master-backed routes: authenticated → has an active org →
 * is a member → that org has a provisioned master. Returns the resolved context (incl. a
 * per-org master client) or a typed error the route turns into an HTTP status.
 * `masterFetch` is injected in tests to stub the master without a live server.
 */
export async function resolveServerContext(
  deps: { auth: Auth; db: Db },
  headers: Headers,
  masterFetch?: FetchLike,
): Promise<ServerContext | { error: ServerContextError }> {
  const auth = await resolveAuth(deps.auth, headers);
  if (!auth) {
    return { error: { status: 401, code: 'unauthorized', message: 'Not authenticated' } };
  }
  if (!auth.activeOrgId) {
    return { error: { status: 403, code: 'no_active_org', message: 'No active organization' } };
  }

  const [org] = await deps.db
    .select()
    .from(organization)
    .where(eq(organization.id, auth.activeOrgId))
    .limit(1);
  if (!org) {
    return {
      error: { status: 403, code: 'org_not_found', message: 'Active organization not found' },
    };
  }

  const [mine] = await deps.db
    .select()
    .from(member)
    .where(and(eq(member.organizationId, org.id), eq(member.userId, auth.user.id)))
    .limit(1);
  if (!mine) {
    return { error: { status: 403, code: 'not_a_member', message: 'Not a member of this org' } };
  }

  if (!org.masterUrl || !org.masterAdminToken) {
    return {
      error: { status: 409, code: 'no_master', message: 'No master provisioned for this org' },
    };
  }

  const master = createMasterClient({
    baseUrl: org.masterUrl,
    adminToken: org.masterAdminToken,
    fetch: masterFetch,
  });

  return {
    user: auth.user,
    org: { id: org.id, name: org.name, slug: org.slug, role: mine.role, hasMaster: true },
    master,
    masterUrl: org.masterUrl,
  };
}

export interface OrgOnlyContext {
  user: SessionUser;
  org: OrgContext;
}

/**
 * Lighter guard for panel-side org resources (webhooks, settings): authenticated → active
 * org → member. Does NOT require a provisioned master, unlike `resolveServerContext`.
 */
export async function resolveOrgContext(
  deps: { auth: Auth; db: Db },
  headers: Headers,
): Promise<OrgOnlyContext | { error: ServerContextError }> {
  const auth = await resolveAuth(deps.auth, headers);
  if (!auth) {
    return { error: { status: 401, code: 'unauthorized', message: 'Not authenticated' } };
  }
  if (!auth.activeOrgId) {
    return { error: { status: 403, code: 'no_active_org', message: 'No active organization' } };
  }
  const org = await resolveOrg(deps.db, auth.user.id, auth.activeOrgId);
  if (!org || org.role === null) {
    return { error: { status: 403, code: 'not_a_member', message: 'Not a member of this org' } };
  }
  return { user: auth.user, org };
}

export function isContextError<T>(
  result: T | { error: ServerContextError },
): result is { error: ServerContextError } {
  return typeof result === 'object' && result !== null && 'error' in result;
}
