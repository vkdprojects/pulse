import { boolean, index, integer, jsonb, pgTable, text, timestamp } from 'drizzle-orm/pg-core';

/**
 * A tenant. Each org routes to its own pulse-master (SaaS isolation model), so the master
 * URL + admin token live on the org record — never in shared env. See docs/admin-panel-design.md §4.
 */
export const organizations = pgTable('organizations', {
  id: text('id').primaryKey(),
  name: text('name').notNull(),
  slug: text('slug').notNull().unique(),
  masterUrl: text('master_url'),
  masterAdminToken: text('master_admin_token'),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
});

export type WebhookScope = 'org' | 'server' | 'user';

/** A customer-configured alert webhook, scoped to the whole org, a single server, or a user. */
export const webhookSubscriptions = pgTable(
  'webhook_subscriptions',
  {
    id: text('id').primaryKey(),
    orgId: text('org_id')
      .notNull()
      .references(() => organizations.id, { onDelete: 'cascade' }),
    scope: text('scope').$type<WebhookScope>().notNull(),
    /** agent id for scope=server, user id for scope=user, null for scope=org. */
    target: text('target'),
    url: text('url').notNull(),
    /** HMAC signing secret for this subscription's deliveries. */
    secret: text('secret').notNull(),
    events: jsonb('events').$type<string[]>().notNull(),
    active: boolean('active').notNull().default(true),
    createdBy: text('created_by').notNull(),
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
      .references(() => organizations.id, { onDelete: 'cascade' }),
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
      .references(() => organizations.id, { onDelete: 'cascade' }),
    actorId: text('actor_id').notNull(),
    action: text('action').notNull(),
    targetType: text('target_type'),
    targetId: text('target_id'),
    metadata: jsonb('metadata').$type<Record<string, unknown>>(),
    createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index('idx_audit_org').on(t.orgId)],
);

export type Organization = typeof organizations.$inferSelect;
export type NewOrganization = typeof organizations.$inferInsert;
export type WebhookSubscription = typeof webhookSubscriptions.$inferSelect;
export type NewWebhookSubscription = typeof webhookSubscriptions.$inferInsert;
export type WebhookDelivery = typeof webhookDeliveries.$inferSelect;
export type AuditLogEntry = typeof auditLog.$inferSelect;
