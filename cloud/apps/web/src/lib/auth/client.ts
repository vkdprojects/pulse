import { createVixpiAuthClient } from '@vixpi/auth/client';
import { env } from '$env/dynamic/public';

/**
 * The single browser-side auth client (better-auth). Exposes signIn/signUp/signOut, the
 * reactive `useSession()` store, and typed `organization` methods. Talks to the API only.
 */
export const authClient = createVixpiAuthClient(env.PUBLIC_API_URL ?? 'http://localhost:3000');

export const { signIn, signUp, signOut, useSession, organization } = authClient;
