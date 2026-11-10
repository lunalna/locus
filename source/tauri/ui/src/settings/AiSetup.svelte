<script module>
    // Opens the wizard, on the key step when a provider is given.
    export const setup = $state({ open: false, provider: null });
</script>

<script>
    import { invoke } from '$backend';
    import { untrack } from 'svelte';

    import { ChevronRight } from '@lucide/svelte';

    import { Button } from '$lib/components/ui/button';
    import * as Dialog from '$lib/components/ui/dialog';
    import { Input } from '$lib/components/ui/input';
    import * as Item from '$lib/components/ui/item';
    import { Label } from '$lib/components/ui/label';
    import { assistant, loadSettings, setSetting } from '../lib/assistant.svelte.js';
    import Choice from './Choice.svelte';

    // 'provider', 'key' or 'model'
    let step = $state('provider');
    let provider = $state(null);
    let key = $state('');
    let url = $state('');
    let models = $state([]);
    let model = $state('');
    let busy = $state(false);
    let error = $state('');

    // Starts over each time the wizard opens.
    $effect.pre(() => {
        if (!setup.open) return;
        const chosen = untrack(() => assistant.settings.providers.find((p) => p.id === setup.provider));
        provider = chosen ?? null;
        step = chosen ? 'key' : 'provider';
        key = '';
        url = chosen?.url && chosen.url !== chosen.defaultUrl ? chosen.url : '';
        error = '';
    });

    function choose(choice) {
        provider = choice;
        url = '';
        step = 'key';
    }

    function check(event) {
        event.preventDefault();
        busy = true;
        invoke('assistant_provider_add', { provider: provider.id, key, url })
            .then((list) => {
                models = list;
                model = list.includes(assistant.settings.model) ? assistant.settings.model : (list[0] ?? '');
                error = '';
                step = 'model';
            })
            .catch((e) => (error = String(e)))
            .finally(() => (busy = false));
    }

    function close() {
        setup.open = false;
        loadSettings();
    }

    function use() {
        busy = true;
        setSetting('provider', provider.id)
            .then(() => setSetting('model', model))
            .then(close)
            .catch((e) => (error = String(e)))
            .finally(() => (busy = false));
    }
</script>

{#snippet problem()}
    {#if error}<p class="text-destructive">{error}</p>{/if}
{/snippet}

<Dialog.Root open={setup.open} onOpenChange={(value) => !value && close()}>
    <Dialog.Content class="max-h-[calc(100vh-4rem)] overflow-auto sm:max-w-md">
        {#if step === 'provider'}
            <Dialog.Header>
                <Dialog.Title>Set up AI</Dialog.Title>
                <Dialog.Description>Choose the provider whose models you want to use. You need an account with them.</Dialog.Description>
            </Dialog.Header>
            <div class="divide-y overflow-hidden rounded-lg border">
                {#each assistant.settings.providers as choice (choice.id)}
                    <Item.Root size="sm" class="w-full cursor-pointer rounded-none text-left hover:bg-muted">
                        {#snippet child({ props })}
                            <button type="button" {...props} onclick={() => choose(choice)}>
                                <Item.Content><Item.Title>{choice.name}</Item.Title></Item.Content>
                                <Item.Actions><ChevronRight class="size-4 text-muted-foreground" /></Item.Actions>
                            </button>
                        {/snippet}
                    </Item.Root>
                {/each}
            </div>
        {:else if step === 'key'}
            <Dialog.Header>
                <Dialog.Title>{provider.name}</Dialog.Title>
                <Dialog.Description>The key is stored on this computer and checked before it is saved.</Dialog.Description>
            </Dialog.Header>
            <form id="ai-setup-key" class="flex flex-col gap-4" onsubmit={check}>
                <div class="flex flex-col gap-2">
                    <Label for="ai-setup-api-key">{provider.needsKey ? 'API key' : 'API key (optional)'}</Label>
                    <Input
                        id="ai-setup-api-key"
                        bind:value={key}
                        type="password"
                        autocomplete="off"
                        placeholder={provider.key ? 'A key is saved. Paste it again to keep using it.' : ''}
                    />
                </div>
                <div class="flex flex-col gap-2">
                    <Label for="ai-setup-url">{provider.defaultUrl ? 'Base URL (optional)' : 'Base URL'}</Label>
                    <Input id="ai-setup-url" type="url" bind:value={url} placeholder={provider.defaultUrl || 'https://...'} />
                </div>
            </form>
            {@render problem()}
            <Dialog.Footer>
                {#if !setup.provider}<Button variant="outline" onclick={() => (step = 'provider')}>Back</Button>{/if}
                <Button type="submit" form="ai-setup-key" disabled={busy}>{busy ? 'Checking...' : 'Next'}</Button>
            </Dialog.Footer>
        {:else}
            <Dialog.Header>
                <Dialog.Title>Choose a model</Dialog.Title>
                <Dialog.Description>{provider.name} is set up. Choose the model the AI features use.</Dialog.Description>
            </Dialog.Header>
            {#if models.length}
                <Choice id="ai-setup-model" value={model} items={models.map((m) => ({ value: m, label: m }))} onchange={(value) => (model = value)} />
            {:else}
                <p class="text-muted-foreground">{provider.name} offers no models with this key.</p>
            {/if}
            {@render problem()}
            <Dialog.Footer>
                <Button variant="outline" onclick={close}>Not now</Button>
                <Button disabled={busy || !model} onclick={use}>Use this model</Button>
            </Dialog.Footer>
        {/if}
    </Dialog.Content>
</Dialog.Root>
