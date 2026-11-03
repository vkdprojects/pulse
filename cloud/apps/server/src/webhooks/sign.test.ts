import { expect, test } from 'vitest';
import { signatureHeader, signPayload, verifySignature } from './sign';

test('signPayload is deterministic and secret-dependent', () => {
  const a = signPayload('secret', '{"x":1}');
  expect(a).toBe(signPayload('secret', '{"x":1}'));
  expect(a).not.toBe(signPayload('other', '{"x":1}'));
  expect(a).toMatch(/^[0-9a-f]{64}$/);
});

test('verifySignature accepts a matching signature and rejects tampering', () => {
  const body = '{"event":"alert.cpu"}';
  const header = signatureHeader('s3cr3t', body);
  expect(header.startsWith('sha256=')).toBe(true);
  expect(verifySignature('s3cr3t', body, header)).toBe(true);
  expect(verifySignature('s3cr3t', '{"event":"tampered"}', header)).toBe(false);
  expect(verifySignature('wrong', body, header)).toBe(false);
});
