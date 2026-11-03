<script lang="ts">
  import type { FleetAgent } from '@vixpi/contracts';
  import { formatUptime } from '$lib/utils/format';
  import StatBar from './StatBar.svelte';
  import StatusDot from './StatusDot.svelte';

  interface Props {
    agents: FleetAgent[];
  }

  let { agents }: Props = $props();
</script>

<div class="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800">
  <table class="w-full min-w-[640px] text-left text-sm">
    <thead class="bg-slate-50 text-xs uppercase text-slate-500 dark:bg-slate-900/50">
      <tr>
        <th class="px-4 py-3 font-medium">Host</th>
        <th class="px-4 py-3 font-medium">Status</th>
        <th class="px-4 py-3 font-medium">CPU</th>
        <th class="px-4 py-3 font-medium">Memory</th>
        <th class="px-4 py-3 font-medium">Uptime</th>
        <th class="px-4 py-3 font-medium">Version</th>
      </tr>
    </thead>
    <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
      {#each agents as agent (agent.id)}
        <tr class="hover:bg-slate-50 dark:hover:bg-slate-900/40">
          <td class="px-4 py-3">
            <a class="font-medium text-indigo-600 hover:underline" href="/servers/{agent.id}">
              {agent.hostname}
            </a>
          </td>
          <td class="px-4 py-3"><StatusDot status={agent.status} /></td>
          <td class="px-4 py-3"><StatBar value={agent.cpuPct} /></td>
          <td class="px-4 py-3"><StatBar value={agent.memPct} /></td>
          <td class="px-4 py-3 tabular-nums text-slate-600 dark:text-slate-400">
            {formatUptime(agent.uptimeSecs)}
          </td>
          <td class="px-4 py-3 text-slate-500">{agent.version || '—'}</td>
        </tr>
      {:else}
        <tr>
          <td class="px-4 py-8 text-center text-slate-500" colspan="6">No servers yet.</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
