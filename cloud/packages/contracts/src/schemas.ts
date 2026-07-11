import { type Static, Type } from '@sinclair/typebox';

/** Standard error envelope returned by the API — never leaks internals. */
export const ErrorResponse = Type.Object({
  error: Type.Object({
    code: Type.String(),
    message: Type.String(),
  }),
});
export type ErrorResponse = Static<typeof ErrorResponse>;

export const HealthResponse = Type.Object({
  status: Type.Literal('ok'),
  service: Type.String(),
  version: Type.String(),
});
export type HealthResponse = Static<typeof HealthResponse>;

export const AgentStatus = Type.Union([Type.Literal('online'), Type.Literal('offline')]);
export type AgentStatus = Static<typeof AgentStatus>;

/** Fleet-overview row for a monitored server. */
export const AgentSummary = Type.Object({
  id: Type.String(),
  hostname: Type.String(),
  status: AgentStatus,
  cpuPct: Type.Number(),
  memPct: Type.Number(),
  version: Type.String(),
});
export type AgentSummary = Static<typeof AgentSummary>;

export const CreateOrganizationBody = Type.Object({
  name: Type.String({ minLength: 1, maxLength: 120 }),
  slug: Type.String({ minLength: 1, maxLength: 60, pattern: '^[a-z0-9-]+$' }),
});
export type CreateOrganizationBody = Static<typeof CreateOrganizationBody>;

export const SessionUser = Type.Object({
  id: Type.String(),
  email: Type.String(),
  name: Type.String(),
});
export type SessionUser = Static<typeof SessionUser>;

export const OrgSummary = Type.Object({
  id: Type.String(),
  name: Type.String(),
  slug: Type.String(),
  /** Caller's role in the org: owner | admin | viewer (null if not a member). */
  role: Type.Union([Type.String(), Type.Null()]),
  hasMaster: Type.Boolean(),
});
export type OrgSummary = Static<typeof OrgSummary>;

export const MeResponse = Type.Object({
  user: SessionUser,
  org: Type.Union([OrgSummary, Type.Null()]),
});
export type MeResponse = Static<typeof MeResponse>;
