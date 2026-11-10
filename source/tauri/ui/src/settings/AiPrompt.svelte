<script>
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';

    let error = $state('');

    const set = (value) =>
        setSetting('prompt', value)
            .then(() => (error = ''))
            .catch((e) => (error = String(e)));

    loadSettings().catch((e) => (error = String(e)));
</script>

{#if !assistant.settings.enabled}
    <p class="text-muted-foreground">AI features are turned off. Turn them on under General.</p>
{:else}
    <p class="mb-4 text-muted-foreground">
        Start the Algebra input with ? to describe what you want in your own words, for example "? circle through A and B". Press Enter and the AI carries it out. It uses the provider and model chosen under AI sidebar.
    </p>

    <div class="flex items-center gap-2">
        <Checkbox id="prompt" checked={assistant.settings.prompt} onCheckedChange={set} />
        <Label for="prompt" class="font-normal">Turn on natural language input</Label>
    </div>
{/if}

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
