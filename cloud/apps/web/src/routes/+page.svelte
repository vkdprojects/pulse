<script lang="ts">
  import { goto } from '$app/navigation';
  import { signOut, useSession } from '$lib/auth/client';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';

  const session = useSession();

  async function logout() {
    await signOut();
    await goto('/login');
  }
</script>

<main
  class="mx-auto flex min-h-full max-w-3xl flex-col items-start gap-6 px-6 py-16
    text-slate-900 dark:text-slate-100"
>
  <div class="flex items-center gap-3">
    <span class="inline-block h-3 w-3 rounded-full bg-emerald-500"></span>
    <h1 class="text-2xl font-semibold tracking-tight">Vixpi Pulse — Control Plane</h1>
  </div>

  {#if $session.isPending}
    <p class="text-slate-500">Loading…</p>
  {:else if $session.data}
    <Card title="Signed in" class="max-w-md">
      <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt class="text-slate-500">Name</dt>
        <dd>{$session.data.user.name}</dd>
        <dt class="text-slate-500">Email</dt>
        <dd>{$session.data.user.email}</dd>
      </dl>
      <div class="mt-5 flex gap-3">
        <Button variant="ghost" onclick={logout}>Sign out</Button>
      </div>
    </Card>
    <p class="text-sm text-slate-500">
      Fleet, metrics and webhooks land in the next milestones (M2+).
    </p>
  {:else}
    <p class="max-w-prose text-slate-600 dark:text-slate-400">
      Multi-tenant control plane for the Vixpi Pulse fleet. Sign in to manage your servers.
    </p>
    <div class="flex gap-3">
      <Button onclick={() => goto('/login')}>Sign in</Button>
      <Button variant="ghost" onclick={() => goto('/register')}>Create account</Button>
    </div>
  {/if}
</main>
