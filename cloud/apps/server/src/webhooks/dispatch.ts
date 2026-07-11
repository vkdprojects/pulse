import type { FetchLike } from '@vixpi/contracts';
import {
  type Db,
  type WebhookSubscription,
  webhookDeliveries,
  webhookSubscriptions,
} from '@vixpi/db';
import { and, desc, eq } from 'drizzle-orm';
import { signatureHeader } from './sign';

export interface AlertEvent {
  event: string;
  /** Agent the alert is about, if any (drives scope=server matching). */
  agentId?: string;
  /** User the alert targets, if any (drives scope=user matching). */
  userId?: string;
  data?: Record<string, unknown>;
}

function scopeMatches(sub: WebhookSubscription, evt: AlertEvent): boolean {
  switch (sub.scope) {
    case 'org':
      return true;
    case 'server':
      return sub.target !== null && sub.target === evt.agentId;
    case 'user':
      return sub.target !== null && sub.target === evt.userId;
    default:
      return false;
  }
}

/** Active subscriptions in an org whose scope + event selector match an alert. */
export async function matchSubscriptions(
  db: Db,
  orgId: string,
  evt: AlertEvent,
): Promise<WebhookSubscription[]> {
  const subs = await db
    .select()
    .from(webhookSubscriptions)
    .where(and(eq(webhookSubscriptions.orgId, orgId), eq(webhookSubscriptions.active, true)));

  return subs.filter((sub) => sub.events.includes(evt.event) && scopeMatches(sub, evt));
}

export interface DeliveryOutcome {
  subscriptionId: string;
  status: 'success' | 'failed';
  responseStatus: number | null;
}

/** Deliver one event to one subscription (HMAC-signed) and record the attempt. */
export async function deliver(
  db: Db,
  sub: WebhookSubscription,
  evt: AlertEvent,
  fetchImpl?: FetchLike,
): Promise<DeliveryOutcome> {
  const body = JSON.stringify({ event: evt.event, data: evt.data ?? {} });
  const doFetch: FetchLike = fetchImpl ?? fetch;

  let status: 'success' | 'failed' = 'failed';
  let responseStatus: number | null = null;
  let error: string | null = null;

  try {
    const res = await doFetch(sub.url, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-vixpi-event': evt.event,
        'x-vixpi-signature': signatureHeader(sub.secret, body),
      },
      body,
    });
    responseStatus = res.status;
    status = res.ok ? 'success' : 'failed';
    if (!res.ok) error = `HTTP ${res.status}`;
  } catch (err) {
    error = err instanceof Error ? err.message : 'delivery failed';
  }

  await db.insert(webhookDeliveries).values({
    id: crypto.randomUUID(),
    subscriptionId: sub.id,
    orgId: sub.orgId,
    event: evt.event,
    status,
    attempts: 1,
    responseStatus,
    error,
  });

  return { subscriptionId: sub.id, status, responseStatus };
}

/** Fan an event out to every matching subscription in an org. */
export async function dispatch(
  db: Db,
  orgId: string,
  evt: AlertEvent,
  fetchImpl?: FetchLike,
): Promise<DeliveryOutcome[]> {
  const subs = await matchSubscriptions(db, orgId, evt);
  return Promise.all(subs.map((sub) => deliver(db, sub, evt, fetchImpl)));
}

/** Recent delivery attempts for a subscription (newest first). */
export async function recentDeliveries(db: Db, subscriptionId: string, limit = 20) {
  return db
    .select()
    .from(webhookDeliveries)
    .where(eq(webhookDeliveries.subscriptionId, subscriptionId))
    .orderBy(desc(webhookDeliveries.createdAt))
    .limit(limit);
}
