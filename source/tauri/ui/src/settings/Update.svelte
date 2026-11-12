<script>
    import { Channel, invoke } from '@tauri-apps/api/core';
    import { X } from '@lucide/svelte';

    import Markdown from '../components/Markdown.svelte';
    import { Button } from '$lib/components/ui/button';

    let { modal } = $props();

    // null until the install starts, then the bytes downloaded and the total when known.
    let progress = $state(null);
    let error = $state('');
    const percent = $derived(progress?.total ? Math.round((progress.downloaded / progress.total) * 100) : null);
    const mb = (bytes) => (bytes / 1e6).toFixed(1);

    const close = () => progress || (modal.update = null);

    async function install() {
        error = '';
        progress = { downloaded: 0, total: null };
        const channel = new Channel();
        channel.onmessage = (message) => (progress = message);
        try {
            await invoke('update_install', { progress: channel });
        } catch (failure) {
            progress = null;
            error = String(failure);
        }
    }
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && close()} />

{#if modal.update}
    <div
        class="fixed inset-0 flex items-center justify-center bg-black/30 font-sans text-[14px] text-foreground"
        role="presentation"
        onclick={(event) => event.target === event.currentTarget && close()}
    >
        <div class="flex max-h-[calc(100vh-32px)] w-[560px] max-w-[calc(100vw-32px)] flex-col rounded-xl bg-background shadow-xl" role="dialog" aria-label="Update available">
            <header class="flex items-center border-b border-border px-5 py-3">
                <h1 class="flex-1 text-[17px] font-semibold">Update available</h1>
                <Button variant="ghost" size="icon-sm" aria-label="Close" disabled={!!progress} onclick={close}><X /></Button>
            </header>
            <div class="flex min-h-0 flex-col gap-4 overflow-auto p-5">
                <p>Locus {modal.update.version} is available. You have {modal.update.current}.</p>
                {#if modal.update.notes}
                    <Markdown text={modal.update.notes} class="rounded-lg border border-border p-3 [--color-line:var(--color-border)] [--color-selected:var(--color-muted)]" />
                {/if}
                {#if error}
                    <p class="text-destructive">{error}</p>
                {/if}
            </div>
            <footer class="flex items-center gap-3 border-t border-border px-5 py-3">
                {#if progress}
                    <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
                        <div class="h-full bg-primary transition-[width]" style:width="{percent ?? 0}%"></div>
                    </div>
                    <span class="text-muted-foreground tabular-nums">
                        {percent === 100 ? 'Installing…' : percent === null ? `${mb(progress.downloaded)} MB` : `${percent}%`}
                    </span>
                {:else}
                    <span class="flex-1"></span>
                    <Button variant="outline" onclick={close}>Later</Button>
                    <Button onclick={install}>Install and restart</Button>
                {/if}
            </footer>
        </div>
    </div>
{/if}
