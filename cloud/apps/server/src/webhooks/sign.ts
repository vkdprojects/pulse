import { createHmac, timingSafeEqual } from 'node:crypto';

/** HMAC-SHA256 of a payload, hex-encoded. Each subscription has its own secret. */
export function signPayload(secret: string, body: string): string {
  return createHmac('sha256', secret).update(body).digest('hex');
}

/** The value sent in the `x-vixpi-signature` header. */
export function signatureHeader(secret: string, body: string): string {
  return `sha256=${signPayload(secret, body)}`;
}

/** Constant-time verification of a hex signature (receiver side). */
export function verifySignature(secret: string, body: string, signature: string): boolean {
  const expected = signPayload(secret, body);
  const a = Buffer.from(expected);
  const b = Buffer.from(signature.replace(/^sha256=/, ''));
  return a.length === b.length && timingSafeEqual(a, b);
}
