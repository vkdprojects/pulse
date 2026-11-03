<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'primary' | 'ghost';

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    children: Snippet;
  }

  let { variant = 'primary', class: className = '', children, ...rest }: Props = $props();

  const variants: Record<Variant, string> = {
    primary: 'bg-indigo-600 text-white hover:bg-indigo-500',
    ghost: 'bg-transparent text-indigo-600 hover:bg-indigo-50 dark:hover:bg-indigo-950',
  };
</script>

<button
  class="inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium
    transition-colors focus-visible:outline-2 focus-visible:outline-offset-2
    focus-visible:outline-indigo-600 disabled:pointer-events-none disabled:opacity-50
    {variants[variant]} {className}"
  {...rest}
>
  {@render children()}
</button>
