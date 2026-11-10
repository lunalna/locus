<script>
    import { Button } from '$lib/components/ui/button';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';
    import { setup } from './AiSetup.svelte';

    let error = $state('');

    const settings = $derived(assistant.settings);
    const provider = $derived(settings.providers.find((provider) => provider.id === settings.provider));

    const set = (value) =>
        setSetting('enabled', value)
            .then(() => (error = ''))
            .catch((e) => (error = String(e)));

    loadSettings().catch((e) => (error = String(e)));
</script>

<p class="mb-4 text-muted-foreground">
    AI features include the AI sidebar, natural language input and the MCP server. When they are turned off, none of them can be used. They are also turned off during exams unless the exam allows them.
</p>

<div class="flex flex-col gap-4">
    <div class="flex items-center gap-2">
        <Checkbox id="enabled" checked={settings.enabled} onCheckedChange={set} />
        <Label for="enabled" class="font-normal">Turn on AI features</Label>
    </div>

    {#if settings.enabled}
        <div class="flex items-center gap-4 rounded-lg border p-3">
            <p class="flex-1">
                {#if settings.ready && settings.model}
                    Using <span class="font-medium">{settings.model}</span> from {provider?.name}.
                {:else}
                    No model is set up yet.
                {/if}
            </p>
            <Button variant="outline" onclick={() => Object.assign(setup, { open: true, provider: null })}>Set up AI</Button>
        </div>
    {/if}
</div>

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
