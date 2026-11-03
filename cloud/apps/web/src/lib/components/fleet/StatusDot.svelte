<script lang="ts">
  interface Props {
    status: string;
  }

  let { status }: Props = $props();
  const online = $derived(status === 'online');
</script>

<span class="inline-flex items-center gap-1.5 text-xs">
  <span
    class="inline-block h-2.5 w-2.5 rounded-full {online
      ? 'bg-emerald-500'
      : 'bg-slate-400 dark:bg-slate-600'}"
  ></span>
  <span class="text-slate-600 dark:text-slate-400">{online ? 'online' : 'offline'}</span>
</span>
