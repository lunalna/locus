<script>
    import { invoke } from '$backend';

    import { Button } from '$lib/components/ui/button';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { Textarea } from '$lib/components/ui/textarea';
    import { loadSettings } from '../lib/assistant.svelte.js';
    import Choice from './Choice.svelte';

    let settings = $state(null);
    let key = $state('');
    let saving = $state(false);
    let error = $state('');

    const load = () => invoke('assistant_settings').then((value) => (settings = value));

    // The assistant view keeps its own copy of the settings, e.g. whether a key is saved.
    const set = (name, value) =>
        invoke('assistant_set', { name, value: String(value) })
            .then(load)
            .then(loadSettings)
            .then(() => (error = ''))
            .catch((e) => (error = String(e)));

    function saveKey(event) {
        event.preventDefault();
        if (!key.trim()) return;
        saving = true;
        set('key', key).then(() => {
            if (!error) key = '';
            saving = false;
        });
    }

    load().catch((e) => (error = String(e)));

    const models = [
        { value: 'claude-haiku-5-5', label: 'Claude Haiku 5.5' },
        { value: 'claude-sonnet-5-5', label: 'Claude Sonnet 5.5' },
        { value: 'claude-opus-5-5', label: 'Claude Opus 5.5' },
        { value: 'claude-fable-5-1', label: 'Claude Fable 5.1' },
    ];
    const efforts = [
        { value: 'low', label: 'Low' },
        { value: 'medium', label: 'Medium' },
        { value: 'high', label: 'High' },
        { value: 'xhigh', label: 'Extra high' },
        { value: 'max', label: 'Max' },
    ];
</script>

<p class="mb-4 text-muted-foreground">
    The AI assistant shows in the app once it is turned on and an API key is saved. It is hidden during exams. The API key is stored on this computer.
</p>

{#if settings}
    <div class="flex flex-col gap-4">
        <div class="flex items-center gap-2">
            <Checkbox id="enabled" checked={settings.enabled} onCheckedChange={(checked) => set('enabled', checked)} />
            <Label for="enabled" class="font-normal">Turn on the AI assistant</Label>
        </div>

        <form class="flex flex-col gap-2" onsubmit={saveKey}>
            <Label for="key">Anthropic API key</Label>
            <div class="flex gap-2">
                <Input
                    id="key"
                    bind:value={key}
                    type="password"
                    autocomplete="off"
                    placeholder={settings.key ? 'A key is saved. Paste a new one to replace it.' : 'sk-ant-...'}
                    class="flex-1"
                />
                <Button type="submit" variant="outline" disabled={saving}>{saving ? 'Checking...' : 'Save'}</Button>
                {#if settings.key}
                    <Button variant="destructive" onclick={() => set('key', '')}>Remove</Button>
                {/if}
            </div>
        </form>

        <div class="flex flex-col gap-2">
            <Label for="url">Base URL</Label>
            <Input id="url" type="url" value={settings.url} onchange={(e) => set('url', e.target.value)} />
        </div>

        <div class="flex flex-col gap-2">
            <Label for="model">Model</Label>
            <Choice id="model" value={settings.model} items={models} onchange={(value) => set('model', value)} />
        </div>

        <div class="flex flex-col gap-2">
            <Label for="effort">Thinking effort</Label>
            <Choice id="effort" value={settings.effort} items={efforts} onchange={(value) => set('effort', value)} />
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
