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

/** Fleet-overview row for a monitored server (normalized from the master by the API). */
export const FleetAgent = Type.Object({
  id: Type.String(),
  hostname: Type.String(),
  status: Type.String(),
  version: Type.String(),
  cpuPct: Type.Union([Type.Number(), Type.Null()]),
  memPct: Type.Union([Type.Number(), Type.Null()]),
  uptimeSecs: Type.Union([Type.Number(), Type.Null()]),
});
export type FleetAgent = Static<typeof FleetAgent>;

export const FleetOverview = Type.Object({
  totalAgents: Type.Integer(),
  online: Type.Integer(),
  offline: Type.Integer(),
  agents: Type.Array(FleetAgent),
});
export type FleetOverview = Static<typeof FleetOverview>;

export const ExecResult = Type.Object({
  agentId: Type.String(),
  exitCode: Type.Integer(),
  stdout: Type.String(),
  stderr: Type.String(),
  timedOut: Type.Boolean(),
});
export type ExecResult = Static<typeof ExecResult>;

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

export const WebhookScope = Type.Union([
  Type.Literal('org'),
  Type.Literal('server'),
  Type.Literal('user'),
]);
export type WebhookScope = Static<typeof WebhookScope>;

export const WebhookSubscription = Type.Object({
  id: Type.String(),
  scope: WebhookScope,
  target: Type.Union([Type.String(), Type.Null()]),
  url: Type.String(),
  events: Type.Array(Type.String()),
  active: Type.Boolean(),
  createdAt: Type.String(),
});
export type WebhookSubscription = Static<typeof WebhookSubscription>;

export const WebhookList = Type.Object({ webhooks: Type.Array(WebhookSubscription) });
export type WebhookList = Static<typeof WebhookList>;

/** Create response includes the signing secret — shown once, never returned again. */
export const CreatedWebhook = Type.Composite([
  WebhookSubscription,
  Type.Object({ secret: Type.String() }),
]);
export type CreatedWebhook = Static<typeof CreatedWebhook>;

export const CreateWebhookBody = Type.Object({
  scope: WebhookScope,
  target: Type.Optional(Type.String()),
  url: Type.String({ minLength: 1 }),
  events: Type.Array(Type.String({ minLength: 1 }), { minItems: 1 }),
});
export type CreateWebhookBody = Static<typeof CreateWebhookBody>;

export const WebhookDelivery = Type.Object({
  id: Type.String(),
  event: Type.String(),
  status: Type.String(),
  responseStatus: Type.Union([Type.Integer(), Type.Null()]),
  error: Type.Union([Type.String(), Type.Null()]),
  createdAt: Type.String(),
});
export type WebhookDelivery = Static<typeof WebhookDelivery>;

export const WebhookDeliveryList = Type.Object({ deliveries: Type.Array(WebhookDelivery) });
export type WebhookDeliveryList = Static<typeof WebhookDeliveryList>;
