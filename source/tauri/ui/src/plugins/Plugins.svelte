<script>
    import { X } from '@lucide/svelte';

    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';

    let { ui, run } = $props();

    let search = $state('');
    let input = $state(null);

    const panel = $derived(ui.plugins.find((plugin) => plugin.id === ui.panel));

    // Everything the running plugins offer: their windows, panels and commands.
    const entries = $derived(
        ui.plugins.flatMap((plugin) => [
            ...(plugin.window ? [{ label: `Show or hide ${plugin.name}`, action: () => run(plugin.id) }] : []),
            ...(plugin.panel ? [{ label: `Open ${plugin.panel.title}`, action: () => (ui.panel = plugin.id) }] : []),
            ...plugin.commands.map((command) => ({
                label: `${plugin.name}: ${command.title}`,
                action: () => run(plugin.id, command.id),
            })),
        ]),
    );
    const shown = $derived(entries.filter((entry) => entry.label.toLowerCase().includes(search.toLowerCase())));

    function choose(entry) {
        ui.list = false;
        search = '';
        entry?.action();
    }

    $effect(() => {
        if (ui.list) input?.focus();
    });
</script>

<div class="font-sans text-[14px] text-foreground">
    {#if panel}
        <aside class="fixed inset-y-0 right-0 flex flex-col border-l border-border bg-background shadow-xl" style:width="{panel.panel.width}px">
            <header class="flex items-center gap-1 border-b border-border px-3 py-2">
                <h2 class="flex-1 truncate font-semibold">{panel.panel.title}</h2>
                <Button variant="ghost" size="icon-sm" aria-label="Close" onclick={() => (ui.panel = null)}><X /></Button>
            </header>
            {#if panel.commands.length}
                <div class="flex flex-wrap gap-1 border-b border-border px-3 py-2" role="toolbar">
                    {#each panel.commands as command (command.id)}
                        <Button variant="outline" size="xs" onclick={() => run(panel.id, command.id)}>{command.title}</Button>
                    {/each}
                </div>
            {/if}
            {#key panel.id}
                <iframe title={panel.panel.title} src={panel.panel.url} class="min-h-0 w-full flex-1 border-0"></iframe>
            {/key}
        </aside>
    {/if}

    {#if ui.list}
        <div
            class="fixed inset-0 flex items-start justify-center bg-black/30 pt-[15vh]"
            role="presentation"
            onclick={(event) => event.target === event.currentTarget && (ui.list = false)}
        >
            <div class="w-[520px] max-w-[calc(100vw-32px)] rounded-xl bg-background p-2 shadow-xl" role="dialog" aria-label="Plugins">
                <Input
                    bind:ref={input}
                    bind:value={search}
                    placeholder="Search plugin commands"
                    onkeydown={(event) => {
                        if (event.key === 'Enter') choose(shown[0]);
                        if (event.key === 'Escape') ui.list = false;
                    }}
                />
                <div class="mt-2 max-h-[50vh] overflow-auto">
                    {#each shown as entry}
                        <button type="button" class="block w-full rounded-md px-3 py-2 text-left hover:bg-accent" onclick={() => choose(entry)}>
                            {entry.label}
                        </button>
                    {:else}
                        <p class="px-3 py-2 text-muted-foreground">
                            {entries.length ? 'No matching commands.' : 'No running plugin offers commands. Plugins are managed in Locus settings.'}
                        </p>
                    {/each}
                </div>
            </div>
        </div>
    {/if}
</div>
