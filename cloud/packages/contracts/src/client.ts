import type { Static, TSchema } from '@sinclair/typebox';
import { Value } from '@sinclair/typebox/value';
import { ErrorResponse, HealthResponse } from './schemas';

/** Minimal fetch signature — satisfied by the global `fetch` and by test mocks alike. */
export type FetchLike = (input: string | URL | Request, init?: RequestInit) => Promise<Response>;

export interface ApiClientOptions {
  baseUrl: string;
  /** Injected for tests / SSR; defaults to the global `fetch`. */
  fetch?: FetchLike;
  /** Extra headers per request (e.g. an API key); resolved lazily. */
  headers?: () => Record<string, string> | Promise<Record<string, string>>;
}

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * The single, typed entry point the web app uses to talk to the API. Responses are validated
 * against the shared contracts, so a drifting server surfaces as a typed error, not `any`.
 */
export function createApiClient(options: ApiClientOptions) {
  const doFetch: FetchLike = options.fetch ?? fetch;
  const base = options.baseUrl.replace(/\/+$/, '');

  async function request<T extends TSchema>(
    path: string,
    schema: T,
    init?: RequestInit,
  ): Promise<Static<T>> {
    const headers = { 'content-type': 'application/json', ...(await options.headers?.()) };
    const res = await doFetch(`${base}${path}`, { credentials: 'include', ...init, headers });
    const body: unknown = await res.json().catch(() => null);

    if (!res.ok) {
      if (Value.Check(ErrorResponse, body)) {
        throw new ApiError(res.status, body.error.code, body.error.message);
      }
      throw new ApiError(res.status, 'http_error', `Request to ${path} failed (${res.status})`);
    }

    if (!Value.Check(schema, body)) {
      throw new ApiError(res.status, 'invalid_response', `Response from ${path} failed validation`);
    }
    return body;
  }

  return {
    request,
    health: () => request('/health', HealthResponse),
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
