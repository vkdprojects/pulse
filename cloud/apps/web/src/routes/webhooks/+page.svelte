<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { ApiError, type CreatedWebhook, type WebhookSubscription } from '@vixpi/contracts';
  import { api } from '$lib/api';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';
  import WebhookForm from '$lib/components/webhooks/WebhookForm.svelte';
  import { useSession } from '$lib/auth/client';

  const session = useSession();

  let webhooks = $state<WebhookSubscription[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let newSecret = $state<string | null>(null);
  let notice = $state<string | null>(null);

  async function load() {
    try {
      webhooks = (await api.webhooks()).webhooks;
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load webhooks';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  $effect(() => {
    if (!$session.isPending && !$session.data) goto('/login');
  });

  function onCreated(webhook: CreatedWebhook) {
    newSecret = webhook.secret;
    notice = null;
    load();
  }

  async function remove(id: string) {
    await api.deleteWebhook(id);
    await load();
  }

  async function test(id: string) {
    notice = null;
    try {
      const result = (await api.testWebhook(id)) as { status: string };
      notice = `Test delivery: ${result.status}`;
    } catch (err) {
      notice = err instanceof ApiError ? err.message : 'Test failed';
    }
  }
</script>

<main class="mx-auto flex min-h-full max-w-3xl flex-col gap-6 px-6 py-12">
  <header class="flex items-center justify-between">
    <h1 class="text-2xl font-semibold tracking-tight">Alert webhooks</h1>
    <a class="text-sm text-indigo-600 hover:underline" href="/fleet">← Fleet</a>
  </header>

  {#if newSecret}
    <Card title="Signing secret (shown once)">
      <p class="mb-2 text-sm text-slate-600 dark:text-slate-400">
        Store this now — it signs every delivery (<code>x-vixpi-signature</code>) and is never
        shown again.
      </p>
      <code class="block break-all rounded bg-slate-100 p-3 text-xs dark:bg-slate-800">{newSecret}</code>
      <div class="mt-3"><Button variant="ghost" onclick={() => (newSecret = null)}>Dismiss</Button></div>
    </Card>
  {/if}

  <Card title="Add a webhook">
    <WebhookForm oncreated={onCreated} />
  </Card>

  {#if notice}
    <p class="text-sm text-slate-600 dark:text-slate-400">{notice}</p>
  {/if}

  <Card title="Subscriptions">
    {#if loading}
      <p class="text-slate-500">Loading…</p>
    {:else if error}
      <p class="text-sm text-red-600" role="alert">{error}</p>
    {:else if webhooks.length === 0}
      <p class="text-slate-500">No webhooks yet.</p>
    {:else}
      <ul class="divide-y divide-slate-100 dark:divide-slate-800">
        {#each webhooks as hook (hook.id)}
          <li class="flex items-center justify-between gap-4 py-3">
            <div class="min-w-0">
              <p class="truncate text-sm font-medium">{hook.url}</p>
              <p class="text-xs text-slate-500">
                {hook.scope}{hook.target ? ` · ${hook.target}` : ''} · {hook.events.join(', ')}
              </p>
            </div>
            <div class="flex shrink-0 gap-2">
              <Button variant="ghost" onclick={() => test(hook.id)}>Test</Button>
              <Button variant="ghost" onclick={() => remove(hook.id)}>Delete</Button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>
</main>
