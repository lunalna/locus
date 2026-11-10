<script>
    import { SquarePen } from '@lucide/svelte';
    import { BitsConfig } from 'bits-ui';
    import { Button } from '$lib/components/ui/button';
    import * as Tabs from '$lib/components/ui/tabs';
    import { assistant, reset } from '../lib/assistant.svelte.js';
    import Camera from './Camera.svelte';
    import Chat from './Chat.svelte';

    let { container } = $props();
</script>

<!-- Popups render inside the shadow root so they get its styles. -->
<BitsConfig defaultPortalTo={container}>
    <div class="embedded absolute inset-0 flex flex-col bg-background font-sans text-[14px] text-foreground">
        {#if !assistant.settings.available}
            <p class="m-6 text-muted-foreground">The AI assistant is not available.</p>
        {:else}
            <div class="flex items-center gap-2 border-b border-border p-2">
                <Tabs.Root bind:value={assistant.view}>
                    <Tabs.List>
                        <Tabs.Trigger value="conversation">Chat</Tabs.Trigger>
                        <Tabs.Trigger value="camera">Camera</Tabs.Trigger>
                    </Tabs.List>
                </Tabs.Root>
                <span class="flex-1"></span>
                {#if assistant.view === 'conversation'}
                    <Button variant="ghost" size="sm" disabled={assistant.busy} onclick={reset}><SquarePen />New chat</Button>
                {/if}
            </div>
            {#if assistant.view === 'conversation'}
                <Chat />
            {:else}
                <Camera />
            {/if}
        {/if}
    </div>
</BitsConfig>
