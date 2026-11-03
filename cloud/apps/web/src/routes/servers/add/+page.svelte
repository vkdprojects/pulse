<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { ApiError, type InstallInfo } from '@vixpi/contracts';
  import { api } from '$lib/api';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';
  import { useSession } from '$lib/auth/client';

  const session = useSession();

  let info = $state<InstallInfo | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let needsMaster = $state(false);
  let copied = $state(false);

  async function load() {
    try {
      info = await api.installInfo();
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) needsMaster = true;
      else error = err instanceof Error ? err.message : 'Failed to load install info';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  $effect(() => {
    if (!$session.isPending && !$session.data) goto('/login');
  });

  async function copy() {
    if (!info) return;
    await navigator.clipboard.writeText(info.command);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1500);
  }
</script>

<main class="mx-auto flex min-h-full max-w-3xl flex-col gap-6 px-6 py-12">
  <header class="flex items-center justify-between">
    <h1 class="text-2xl font-semibold tracking-tight">Add a server</h1>
    <a class="text-sm text-indigo-600 hover:underline" href="/fleet">← Fleet</a>
  </header>

  {#if loading}
    <p class="text-slate-500">Loading…</p>
  {:else if needsMaster}
    <Card title="No master provisioned yet">
      <p class="text-sm text-slate-600 dark:text-slate-400">
        This organization has no pulse-master connected. A master must be provisioned before
        you can add agents.
      </p>
    </Card>
  {:else if error}
    <Card title="Something went wrong">
      <p class="text-sm text-red-600" role="alert">{error}</p>
    </Card>
  {:else if info}
    <Card title="Run this on the server you want to monitor">
      <p class="mb-3 text-sm text-slate-600 dark:text-slate-400">
        Replace <code>ghp_YOUR_TOKEN</code> with a GitHub token that can download the binaries.
        The agent auto-registers and appears in your fleet within seconds.
      </p>
      <pre class="overflow-x-auto rounded-md bg-slate-900 p-4 text-xs text-slate-100">{info.command}</pre>
      <div class="mt-3 flex items-center gap-3">
        <Button onclick={copy}>{copied ? 'Copied!' : 'Copy command'}</Button>
        <span class="text-xs text-slate-500">Master: {info.masterEndpoint}</span>
      </div>
    </Card>

    <Card title="Register token">
      <p class="mb-2 text-sm text-slate-600 dark:text-slate-400">
        The token above authenticates the agent's first handshake with the master.
      </p>
      <code class="block break-all rounded bg-slate-100 p-3 text-xs dark:bg-slate-800">{info.registerToken}</code>
    </Card>
  {/if}
</main>
