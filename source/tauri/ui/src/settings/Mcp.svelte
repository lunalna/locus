<script>
    import { invoke } from '$backend';

    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { assistant, loadSettings } from '../lib/assistant.svelte.js';

    let settings = $state(null);
    let error = $state('');

    const load = () => invoke('mcp_settings').then((value) => (settings = value));

    const set = (enabled, port) =>
        invoke('mcp_set', { enabled, port: Number(port) })
            .then(() => (error = ''))
            .catch((e) => (error = String(e)))
            .then(load);

    load().catch((e) => (error = String(e)));
    loadSettings();
</script>

{#if !assistant.settings.enabled}
    <p class="text-muted-foreground">AI features are turned off. Turn them on under General.</p>
{:else}
    <p class="mb-4 text-muted-foreground">
        Lets AI tools on this computer, like Claude Code, use the AI sidebar's tools in Locus through the Model Context Protocol. It is turned off during exams.
    </p>

    {#if settings}
        <div class="flex flex-col gap-4">
            <div class="flex items-center gap-2">
                <Checkbox id="enabled" checked={settings.enabled} onCheckedChange={(checked) => set(checked, settings.port)} />
                <Label for="enabled" class="font-normal">Enable the MCP server</Label>
            </div>

            <div class="flex flex-col gap-2">
                <Label for="port">Port</Label>
                <Input id="port" type="number" min="1024" max="65535" value={settings.port} onchange={(e) => set(settings.enabled, e.target.value)} class="w-32" />
            </div>

            {#if settings.running}
                <div class="flex flex-col gap-2">
                    <span class="text-sm font-medium">Add it to Claude Code</span>
                    <code class="rounded-lg border border-input px-2.5 py-1.5 font-mono text-[13px] select-all">claude mcp add --transport http locus http://127.0.0.1:{settings.port}/mcp</code>
                </div>
            {/if}
        </div>
    {/if}
{/if}

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
