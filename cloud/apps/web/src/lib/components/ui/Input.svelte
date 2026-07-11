<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLInputAttributes, 'value'> {
    label: string;
    value?: string;
  }

  let { label, value = $bindable(''), class: className = '', id, ...rest }: Props = $props();
  const inputId = $derived(id ?? `in-${label.toLowerCase().replace(/\s+/g, '-')}`);
</script>

<label class="flex flex-col gap-1.5 text-sm" for={inputId}>
  <span class="font-medium text-slate-700 dark:text-slate-300">{label}</span>
  <input
    id={inputId}
    bind:value
    class="rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm
      transition-colors placeholder:text-slate-400 focus-visible:border-indigo-500
      focus-visible:outline-2 focus-visible:outline-indigo-500 dark:border-slate-700
      dark:bg-slate-900 dark:text-slate-100 {className}"
    {...rest}
  />
</label>
