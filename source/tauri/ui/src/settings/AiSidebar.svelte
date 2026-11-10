<script>
    import { invoke } from '$backend';

    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import { Textarea } from '$lib/components/ui/textarea';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';
    import Choice from './Choice.svelte';

    let models = $state([]);
    let error = $state('');

    const settings = $derived(assistant.settings);
    const provider = $derived(settings.providers.find((provider) => provider.id === settings.provider));

    const set = (name, value) =>
        setSetting(name, value)
            .then(() => (error = ''))
            .catch((e) => (error = String(e)));

    // Loads the models again when another provider is chosen or set up.
    const ready = $derived(provider?.ready ? provider.id : null);
    $effect(() => {
        if (!ready) return;
        models = [];
        invoke('assistant_models', { provider: ready })
            .then((list) => (models = list))
            .catch((e) => (error = String(e)));
    });

    const modelItems = $derived(
        [...new Set([...models, settings.model].filter(Boolean))].map((model) => ({ value: model, label: model })),
    );

    const effortLabels = { default: 'Model default', low: 'Low', medium: 'Medium', high: 'High', xhigh: 'Extra high', max: 'Max' };

    loadSettings().catch((e) => (error = String(e)));
</script>

{#if !settings.enabled}
    <p class="text-muted-foreground">AI features are turned off. Turn them on under General.</p>
{:else if provider}
    <p class="mb-4 text-muted-foreground">
        The AI sidebar shows next to the views once it is turned on and a model is chosen. Plugins that use the AI use these settings too.
    </p>

    <div class="flex flex-col gap-4">
        <div class="flex items-center gap-2">
            <Checkbox id="sidebar" checked={settings.sidebar} onCheckedChange={(checked) => set('sidebar', checked)} />
            <Label for="sidebar" class="font-normal">Show the AI sidebar</Label>
        </div>

        <div class="flex flex-col gap-2">
            <Label for="provider">Provider</Label>
            <Choice
                id="provider"
                value={settings.provider}
                items={settings.providers.map((provider) => ({ value: provider.id, label: provider.name }))}
                onchange={(value) => set('provider', value)}
            />
            {#if !provider.ready}
                <p class="text-muted-foreground">Set up {provider.name} under Model providers first.</p>
            {/if}
        </div>

        <div class="flex flex-col gap-2">
            <Label for="model">Model</Label>
            <Choice id="model" value={settings.model} items={modelItems} disabled={!provider.ready} onchange={(value) => set('model', value)} />
        </div>

        <div class="flex flex-col gap-2">
            <Label for="effort">Thinking effort</Label>
            <Choice
                id="effort"
                value={settings.effort}
                items={provider.efforts.map((effort) => ({ value: effort, label: effortLabels[effort] }))}
                onchange={(value) => set('effort', value)}
            />
        </div>

        <div class="flex items-center gap-2">
            <Checkbox id="thinking" checked={settings.thinking} onCheckedChange={(checked) => set('thinking', checked)} />
            <Label for="thinking" class="font-normal">Show thinking in the chat</Label>
        </div>

        <div class="flex flex-col gap-2">
            <Label for="instructions">Custom instructions</Label>
            <Textarea
                id="instructions"
                rows={5}
                value={settings.instructions}
                placeholder="For example: Explain each step. Answer in German."
                onchange={(e) => set('instructions', e.target.value)}
            />
        </div>
    </div>
{/if}

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
