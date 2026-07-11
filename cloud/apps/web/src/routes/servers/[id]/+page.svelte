<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError } from '@vixpi/contracts';
  import { api } from '$lib/api';
  import StatBar from '$lib/components/fleet/StatBar.svelte';
  import StatusDot from '$lib/components/fleet/StatusDot.svelte';
  import ExecPanel from '$lib/components/servers/ExecPanel.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import { formatUptime } from '$lib/utils/format';
  import { createFleet } from '$lib/hooks/useFleet';

  const fleet = createFleet();
  const id = $derived(page.params.id ?? '');
  const agent = $derived($fleet.data?.agents.find((a) => a.id === id) ?? null);

  let newName = $state('');
  let actionError = $state<string | null>(null);

  onMount(() => fleet.start());
  onDestroy(() => fleet.stop());

  async function rename(event: SubmitEvent) {
    event.preventDefault();
    actionError = null;
    try {
      await api.renameServer(id, newName);
      newName = '';
      await fleet.refresh();
    } catch (err) {
      actionError = err instanceof ApiError ? err.message : 'Rename failed';
    }
  }

  async function remove() {
    actionError = null;
    try {
      await api.removeServer(id);
      await goto('/fleet');
    } catch (err) {
      actionError = err instanceof ApiError ? err.message : 'Remove failed';
    }
  }
</script>

<main class="mx-auto flex min-h-full max-w-3xl flex-col gap-6 px-6 py-12">
  <a class="text-sm text-indigo-600 hover:underline" href="/fleet">← Fleet</a>

  <header class="flex items-center justify-between">
    <h1 class="text-2xl font-semibold tracking-tight">{agent?.hostname ?? id}</h1>
    {#if agent}<StatusDot status={agent.status} />{/if}
  </header>

  {#if actionError}
    <p class="text-sm text-red-600" role="alert">{actionError}</p>
  {/if}

  {#if agent}
    <Card title="Now">
      <dl class="grid grid-cols-[6rem_1fr] items-center gap-x-4 gap-y-3 text-sm">
        <dt class="text-slate-500">CPU</dt>
        <dd><StatBar value={agent.cpuPct} /></dd>
        <dt class="text-slate-500">Memory</dt>
        <dd><StatBar value={agent.memPct} /></dd>
        <dt class="text-slate-500">Uptime</dt>
        <dd class="tabular-nums">{formatUptime(agent.uptimeSecs)}</dd>
        <dt class="text-slate-500">Version</dt>
        <dd>{agent.version || '—'}</dd>
      </dl>
    </Card>
  {:else}
    <p class="text-slate-500">Loading…</p>
  {/if}

  <Card title="Run command">
    <ExecPanel serverId={id} />
  </Card>

  <Card title="Manage">
    <form class="mb-4 flex items-end gap-2" onsubmit={rename}>
      <div class="flex-1"><Input label="Rename" bind:value={newName} placeholder="new-hostname" required /></div>
      <Button type="submit">Save</Button>
    </form>
    <Button variant="ghost" onclick={remove}>Remove from fleet</Button>
  </Card>
</main>
