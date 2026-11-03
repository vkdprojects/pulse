<script lang="ts">
  import { ApiError, type ExecResult } from '@vixpi/contracts';
  import { api } from '$lib/api';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';

  interface Props {
    serverId: string;
  }

  let { serverId }: Props = $props();

  let command = $state('');
  let running = $state(false);
  let error = $state<string | null>(null);
  let result = $state<ExecResult | null>(null);

  async function run(event: SubmitEvent) {
    event.preventDefault();
    running = true;
    error = null;
    result = null;
    try {
      result = (await api.execServer(serverId, command)) as ExecResult;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Command failed';
    } finally {
      running = false;
    }
  }
</script>

<form class="flex flex-col gap-3" onsubmit={run}>
  <div class="flex items-end gap-2">
    <div class="flex-1">
      <Input label="Command" bind:value={command} placeholder="uptime" required />
    </div>
    <Button type="submit" disabled={running || command.length === 0}>
      {running ? 'Running…' : 'Run'}
    </Button>
  </div>

  {#if error}
    <p class="text-sm text-red-600" role="alert">{error}</p>
  {/if}

  {#if result}
    <div class="rounded-md border border-slate-200 dark:border-slate-800">
      <div class="border-b border-slate-200 px-3 py-1.5 text-xs text-slate-500 dark:border-slate-800">
        exit code {result.exitCode}{result.timedOut ? ' · timed out' : ''}
      </div>
      <pre class="max-h-64 overflow-auto px-3 py-2 text-xs text-slate-800 dark:text-slate-200">{result.stdout || result.stderr || '(no output)'}</pre>
    </div>
  {/if}
</form>
