<script lang="ts">
  import { ApiError, type CreatedWebhook, type WebhookScope } from '@vixpi/contracts';
  import { api } from '$lib/api';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';

  interface Props {
    oncreated: (webhook: CreatedWebhook) => void;
  }

  let { oncreated }: Props = $props();

  const allEvents = ['alert.cpu', 'alert.mem', 'alert.disk'];

  let scope = $state<WebhookScope>('org');
  let target = $state('');
  let url = $state('');
  let events = $state<string[]>(['alert.cpu']);
  let error = $state<string | null>(null);
  let saving = $state(false);

  function toggle(event: string) {
    events = events.includes(event) ? events.filter((e) => e !== event) : [...events, event];
  }

  async function submit(ev: SubmitEvent) {
    ev.preventDefault();
    saving = true;
    error = null;
    try {
      const created = await api.createWebhook({
        scope,
        target: scope === 'org' ? undefined : target || undefined,
        url,
        events,
      });
      oncreated(created);
      url = '';
      target = '';
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Failed to create webhook';
    } finally {
      saving = false;
    }
  }
</script>

<form class="flex flex-col gap-4" onsubmit={submit}>
  <label class="flex flex-col gap-1.5 text-sm">
    <span class="font-medium text-slate-700 dark:text-slate-300">Scope</span>
    <select
      bind:value={scope}
      class="rounded-md border border-slate-300 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
    >
      <option value="org">Whole organization</option>
      <option value="server">A single server</option>
      <option value="user">A single user</option>
    </select>
  </label>

  {#if scope !== 'org'}
    <Input label={scope === 'server' ? 'Server (agent) id' : 'User id'} bind:value={target} required />
  {/if}

  <Input label="Endpoint URL" type="url" bind:value={url} placeholder="https://…" required />

  <fieldset class="flex flex-col gap-2">
    <span class="text-sm font-medium text-slate-700 dark:text-slate-300">Events</span>
    <div class="flex flex-wrap gap-3">
      {#each allEvents as event (event)}
        <label class="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={events.includes(event)} onchange={() => toggle(event)} />
          {event}
        </label>
      {/each}
    </div>
  </fieldset>

  {#if error}
    <p class="text-sm text-red-600" role="alert">{error}</p>
  {/if}

  <Button type="submit" disabled={saving || events.length === 0}>
    {saving ? 'Creating…' : 'Add webhook'}
  </Button>
</form>
