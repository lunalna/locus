<script>
    import { ChevronDown } from '@lucide/svelte';

    import { Badge } from '$lib/components/ui/badge';
    import { Button } from '$lib/components/ui/button';
    import * as Collapsible from '$lib/components/ui/collapsible';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';

    // Per provider: the key being typed, whether it is being checked, and the last error.
    let keys = $state({});
    let saving = $state({});
    let errors = $state({});

    const set = (provider, name, value) =>
        setSetting(name, value, provider.id)
            .then(() => (errors[provider.id] = ''))
            .catch((e) => (errors[provider.id] = String(e)));

    function saveKey(event, provider) {
        event.preventDefault();
        if (!keys[provider.id]?.trim()) return;
        saving[provider.id] = true;
        set(provider, 'key', keys[provider.id]).then(() => {
            if (!errors[provider.id]) keys[provider.id] = '';
            saving[provider.id] = false;
        });
    }

    loadSettings();
</script>

{#if !assistant.settings.enabled}
    <p class="text-muted-foreground">AI features are turned off. Turn them on under General.</p>
{:else}
    <p class="mb-4 text-muted-foreground">
        Set up the providers whose models you want to use. API keys are stored on this computer and checked when saved.
    </p>

    <div class="divide-y overflow-hidden rounded-lg border">
        {#each assistant.settings.providers as provider (provider.id)}
            <Collapsible.Root>
                <Collapsible.Trigger class="group flex w-full items-center gap-2.5 px-3 py-2.5 text-left hover:bg-muted">
                    <span class="flex-1 font-medium">{provider.name}</span>
                    {#if provider.ready}<Badge variant="secondary">Set up</Badge>{/if}
                    <ChevronDown class="size-4 transition-transform group-data-[state=open]:rotate-180" />
                </Collapsible.Trigger>
                <Collapsible.Content class="flex flex-col gap-4 border-t px-3 py-3">
                    <form class="flex flex-col gap-2" onsubmit={(event) => saveKey(event, provider)}>
                        <Label for="{provider.id}-key">{provider.needsKey ? 'API key' : 'API key (optional)'}</Label>
                        <div class="flex gap-2">
                            <Input
                                id="{provider.id}-key"
                                bind:value={keys[provider.id]}
                                type="password"
                                autocomplete="off"
                                placeholder={provider.key ? 'A key is saved. Paste a new one to replace it.' : ''}
                                class="flex-1"
                            />
                            <Button type="submit" variant="outline" disabled={saving[provider.id]}>{saving[provider.id] ? 'Checking...' : 'Save'}</Button>
                            {#if provider.key}
                                <Button variant="destructive" onclick={() => set(provider, 'key', '')}>Remove</Button>
                            {/if}
                        </div>
                    </form>
                    <div class="flex flex-col gap-2">
                        <Label for="{provider.id}-url">Base URL</Label>
                        <Input
                            id="{provider.id}-url"
                            type="url"
                            value={provider.url}
                            placeholder={provider.defaultUrl || 'https://...'}
                            onchange={(e) => set(provider, 'url', e.target.value)}
                        />
                    </div>
                    {#if errors[provider.id]}
                        <p class="text-destructive">{errors[provider.id]}</p>
                    {/if}
                </Collapsible.Content>
            </Collapsible.Root>
        {/each}
    </div>
{/if}
