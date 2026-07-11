<script lang="ts">
  import { goto } from '$app/navigation';
  import { signIn } from '$lib/auth/client';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';
  import Input from '$lib/components/ui/Input.svelte';

  let email = $state('');
  let password = $state('');
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    loading = true;
    error = null;
    const res = await signIn.email({ email, password });
    loading = false;
    if (res.error) {
      error = res.error.message ?? 'Sign-in failed';
      return;
    }
    await goto('/');
  }
</script>

<main class="mx-auto flex min-h-full max-w-md flex-col justify-center px-6 py-16">
  <Card title="Sign in">
    <form class="flex flex-col gap-4" onsubmit={submit}>
      <Input label="Email" type="email" bind:value={email} required autocomplete="email" />
      <Input
        label="Password"
        type="password"
        bind:value={password}
        required
        autocomplete="current-password"
      />
      {#if error}
        <p class="text-sm text-red-600" role="alert">{error}</p>
      {/if}
      <Button type="submit" disabled={loading}>{loading ? 'Signing in…' : 'Sign in'}</Button>
    </form>
    <p class="mt-4 text-sm text-slate-600 dark:text-slate-400">
      No account? <a class="text-indigo-600 hover:underline" href="/register">Create one</a>
    </p>
  </Card>
</main>
