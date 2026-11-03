<script lang="ts">
  import { goto } from '$app/navigation';
  import { organization, signUp } from '$lib/auth/client';
  import Button from '$lib/components/ui/Button.svelte';
  import Card from '$lib/components/ui/Card.svelte';
  import Input from '$lib/components/ui/Input.svelte';

  let name = $state('');
  let email = $state('');
  let password = $state('');
  let orgName = $state('');
  let error = $state<string | null>(null);
  let loading = $state(false);

  // Slug derived from the org name — the tenant identifier.
  const slug = $derived(
    orgName
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, ''),
  );

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    loading = true;
    error = null;

    const signed = await signUp.email({ email, password, name });
    if (signed.error) {
      loading = false;
      error = signed.error.message ?? 'Sign-up failed';
      return;
    }

    // SaaS: the new user owns a fresh organization (design §5).
    const created = await organization.create({ name: orgName, slug });
    if (created.error) {
      loading = false;
      error = created.error.message ?? 'Could not create organization';
      return;
    }
    if (created.data) {
      await organization.setActive({ organizationId: created.data.id });
    }

    loading = false;
    await goto('/');
  }
</script>

<main class="mx-auto flex min-h-full max-w-md flex-col justify-center px-6 py-16">
  <Card title="Create your account">
    <form class="flex flex-col gap-4" onsubmit={submit}>
      <Input label="Your name" bind:value={name} required autocomplete="name" />
      <Input label="Email" type="email" bind:value={email} required autocomplete="email" />
      <Input
        label="Password"
        type="password"
        bind:value={password}
        required
        minlength={8}
        autocomplete="new-password"
      />
      <Input label="Organization name" bind:value={orgName} required />
      {#if slug}
        <p class="-mt-2 text-xs text-slate-500">Organization URL: <code>/{slug}</code></p>
      {/if}
      {#if error}
        <p class="text-sm text-red-600" role="alert">{error}</p>
      {/if}
      <Button type="submit" disabled={loading}>
        {loading ? 'Creating…' : 'Create account'}
      </Button>
    </form>
    <p class="mt-4 text-sm text-slate-600 dark:text-slate-400">
      Already have an account? <a class="text-indigo-600 hover:underline" href="/login">Sign in</a>
    </p>
  </Card>
</main>
