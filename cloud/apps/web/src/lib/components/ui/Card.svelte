<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    children: Snippet;
    class?: string;
  }

  let { title, children, class: className = '' }: Props = $props();
</script>

<section
  class="w-full rounded-xl border border-slate-200 bg-white p-6 shadow-sm
    dark:border-slate-800 dark:bg-slate-900 {className}"
>
  {#if title}
    <h2 class="mb-4 text-lg font-semibold text-slate-900 dark:text-slate-100">{title}</h2>
  {/if}
  {@render children()}
</section>
