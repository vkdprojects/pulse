<script lang="ts">
  import { formatPct } from '$lib/utils/format';

  interface Props {
    value: number | null;
  }

  let { value }: Props = $props();

  const width = $derived(value === null ? 0 : Math.max(0, Math.min(100, value)));
  const tone = $derived(width >= 90 ? 'bg-red-500' : width >= 70 ? 'bg-amber-500' : 'bg-emerald-500');
</script>

<div class="flex items-center gap-2">
  <div class="h-2 w-24 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800">
    <div class="h-full rounded-full {tone}" style="width: {width}%"></div>
  </div>
  <span class="w-10 text-right text-xs tabular-nums text-slate-600 dark:text-slate-400">
    {formatPct(value)}
  </span>
</div>
