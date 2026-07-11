<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { useSession } from '$lib/auth/client';
  import ServerTable from '$lib/components/fleet/ServerTable.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';
  import { createFleet } from '$lib/hooks/useFleet';

  const session = useSession();
  const fleet = createFleet();

  onMount(() => fleet.start());
  onDestroy(() => fleet.stop());

  $effect(() => {
    if (!$session.isPending && !$session.data) goto('/login');
  });
</script>

<main class="mx-auto flex min-h-full max-w-5xl flex-col gap-6 px-6 py-12">
  <header class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">Fleet</h1>
      {#if $fleet.data}
        <p class="text-sm text-slate-500">
          {$fleet.data.online} online · {$fleet.data.offline} offline · {$fleet.data.totalAgents} total
        </p>
      {/if}
    </div>
    <div class="flex items-center gap-3">
      <a class="text-sm text-indigo-600 hover:underline" href="/servers/add">Add server</a>
      <a class="text-sm text-indigo-600 hover:underline" href="/webhooks">Webhooks</a>
      <Button variant="ghost" onclick={() => fleet.refresh()}>Refresh</Button>
    </div>
  </header>

  {#if $fleet.loading}
    <p class="text-slate-500">Loading fleet…</p>
  {:else if $fleet.needsMaster}
    <Card title="No master provisioned yet">
      <p class="text-sm text-slate-600 dark:text-slate-400">
        This organization has no pulse-master connected. Adding servers (install command +
        register token) lands in the next milestone.
      </p>
    </Card>
  {:else if $fleet.error}
    <Card title="Couldn’t load the fleet">
      <p class="text-sm text-red-600" role="alert">{$fleet.error}</p>
    </Card>
  {:else if $fleet.data}
    <ServerTable agents={$fleet.data.agents} />
  {/if}
</main>
