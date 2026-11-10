<script>
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';

    let error = $state('');

    const set = (value) =>
        setSetting('enabled', value)
            .then(() => (error = ''))
            .catch((e) => (error = String(e)));

    loadSettings().catch((e) => (error = String(e)));
</script>

<p class="mb-4 text-muted-foreground">
    AI features include the AI sidebar and the MCP server. When they are turned off, the sidebar is hidden and the MCP server stops. They are also turned off during exams unless the exam allows them.
</p>

<div class="flex items-center gap-2">
    <Checkbox id="enabled" checked={assistant.settings.enabled} onCheckedChange={set} />
    <Label for="enabled" class="font-normal">Turn on AI features</Label>
</div>

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
