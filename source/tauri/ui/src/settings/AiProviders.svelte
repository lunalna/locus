<script>
    import { Button } from '$lib/components/ui/button';
    import * as Item from '$lib/components/ui/item';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';
    import AddItem from './AddItem.svelte';
    import { setup } from './AiSetup.svelte';

    let error = $state('');

    const added = $derived(assistant.settings.providers.filter((provider) => provider.added));

    const remove = (provider) =>
        setSetting('remove', '', provider.id)
            .then(() => (error = ''))
            .catch((e) => (error = String(e)));

    const open = (provider = null) => Object.assign(setup, { open: true, provider });

    loadSettings();
</script>

{#if !assistant.settings.enabled}
    <p class="text-muted-foreground">AI features are turned off. Turn them on under General.</p>
{:else}
    <p class="mb-4 text-muted-foreground">The providers whose models you can use. API keys are stored on this computer.</p>

    <div class="divide-y overflow-hidden rounded-lg border">
        {#each added as provider (provider.id)}
            <Item.Root size="sm" class="rounded-none">
                <Item.Content>
                    <Item.Title>{provider.name}</Item.Title>
                    <Item.Description>{provider.url}</Item.Description>
                </Item.Content>
                <Item.Actions>
                    <Button variant="outline" size="sm" onclick={() => open(provider.id)}>Change</Button>
                    <Button variant="destructive" size="sm" onclick={() => remove(provider)}>Remove</Button>
                </Item.Actions>
            </Item.Root>
        {/each}
        <AddItem label="provider" onclick={() => open()} />
    </div>
{/if}

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
