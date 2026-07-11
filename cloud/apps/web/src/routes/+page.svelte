<script lang="ts">
  import Button from '$lib/components/ui/Button.svelte';
  import { theme } from '$lib/stores/theme';
</script>

<main
  class="mx-auto flex min-h-full max-w-3xl flex-col items-start gap-6 px-6 py-16
    text-slate-900 dark:text-slate-100"
>
  <div class="flex items-center gap-3">
    <span class="inline-block h-3 w-3 rounded-full bg-emerald-500"></span>
    <h1 class="text-2xl font-semibold tracking-tight">Vixpi Pulse — Control Plane</h1>
  </div>

  <p class="max-w-prose text-slate-600 dark:text-slate-400">
    Multi-tenant control plane for the Vixpi Pulse fleet. This is the M0 scaffold — auth,
    fleet, metrics and webhooks land in the next milestones.
  </p>

  <div class="flex items-center gap-3">
    <Button onclick={() => theme.toggle()}>Toggle theme ({$theme})</Button>
    <Button variant="ghost" onclick={() => window.location.assign('http://localhost:3000/docs')}>
      API docs
    </Button>
  </div>
</main>
