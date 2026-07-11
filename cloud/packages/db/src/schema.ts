import { boolean, index, integer, jsonb, pgTable, text, timestamp } from 'drizzle-orm/pg-core';
import { organization, user } from './auth-schema';

// better-auth owns user / session / account / verification / organization / member / invitation
// (generated in auth-schema.ts). The `organization` table is the tenant and carries per-org
// master routing (masterUrl / masterAdminToken). Everything below is control-plane domain data
// keyed to that org.
export * from './auth-schema';

export type WebhookScope = 'org' | 'server' | 'user';

/** A customer-configured alert webhook, scoped to the whole org, a single server, or a user. */
export const webhookSubscriptions = pgTable(
  'webhook_subscriptions',
  {
    id: text('id').primaryKey(),
    orgId: text('org_id')
      .notNull()
      .references(() => organization.id, { onDelete: 'cascade' }),
    scope: text('scope').$type<WebhookScope>().notNull(),
    /** agent id for scope=server, user id for scope=user, null for scope=org. */
    target: text('target'),
    url: text('url').notNull(),
    /** HMAC signing secret for this subscription's deliveries. */
    secret: text('secret').notNull(),
    events: jsonb('events').$type<string[]>().notNull(),
    active: boolean('active').notNull().default(true),
    createdBy: text('created_by').references(() => user.id, { onDelete: 'set null' }),
    createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index('idx_webhook_subs_org').on(t.orgId)],
);

export type WebhookDeliveryStatus = 'pending' | 'success' | 'failed';

/** Delivery attempt log for a webhook subscription — powers the "recent deliveries" UI. */
export const webhookDeliveries = pgTable(
  'webhook_deliveries',
  {
    id: text('id').primaryKey(),
    subscriptionId: text('subscription_id')
      .notNull()
      .references(() => webhookSubscriptions.id, { onDelete: 'cascade' }),
    orgId: text('org_id')
      .notNull()
      .references(() => organization.id, { onDelete: 'cascade' }),
    event: text('event').notNull(),
    status: text('status').$type<WebhookDeliveryStatus>().notNull().default('pending'),
    attempts: integer('attempts').notNull().default(0),
    responseStatus: integer('response_status'),
    error: text('error'),
    createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index('idx_webhook_deliveries_sub').on(t.subscriptionId)],
);

/** Immutable audit trail for state-changing actions (esp. remote exec). */
export const auditLog = pgTable(
  'audit_log',
  {
    id: text('id').primaryKey(),
    orgId: text('org_id')
      .notNull()
      .references(() => organization.id, { onDelete: 'cascade' }),
    actorId: text('actor_id').references(() => user.id, { onDelete: 'set null' }),
    action: text('action').notNull(),
    targetType: text('target_type'),
    targetId: text('target_id'),
    metadata: jsonb('metadata').$type<Record<string, unknown>>(),
    createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index('idx_audit_org').on(t.orgId)],
);

export type Organization = typeof organization.$inferSelect;
export type User = typeof user.$inferSelect;
export type WebhookSubscription = typeof webhookSubscriptions.$inferSelect;
export type NewWebhookSubscription = typeof webhookSubscriptions.$inferInsert;
export type WebhookDelivery = typeof webhookDeliveries.$inferSelect;
export type AuditLogEntry = typeof auditLog.$inferSelect;
