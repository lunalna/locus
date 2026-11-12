<script>
    import { X } from '@lucide/svelte';

    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';

    let { ui, run, toolInput, wheel } = $props();

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
    {#if ui.tool && ui.view}
        <div
            class="fixed cursor-crosshair touch-none"
            style:left="{ui.view.left}px"
            style:top="{ui.view.top}px"
            style:width="{ui.view.width}px"
            style:height="{ui.view.height}px"
            role="application"
            onpointerdown={toolInput}
            onpointermove={toolInput}
            onpointerup={toolInput}
            onwheel={wheel}
            oncontextmenu={(event) => event.preventDefault()}
        ></div>
    {/if}

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
                <iframe title={panel.panel.title} src={panel.panel.url} data-plugin={panel.id} class="min-h-0 w-full flex-1 border-0"></iframe>
            {/key}
        </aside>
    {/if}

    {#each ui.plugins.filter((plugin) => plugin.overlay) as plugin (plugin.id)}
        <iframe title={plugin.name} src={plugin.overlay} data-plugin={plugin.id} class="pointer-events-none fixed inset-0 h-full w-full border-0"></iframe>
    {/each}

    {#if ui.toasts.length}
        <div class="fixed bottom-4 left-1/2 flex -translate-x-1/2 flex-col gap-2">
            {#each ui.toasts as toast (toast.id)}
                <div class="w-[360px] max-w-[calc(100vw-32px)] rounded-lg border border-border bg-background px-4 py-3 shadow-lg" role="status">
                    <p class="font-semibold">{toast.name}</p>
                    <p>{toast.text}</p>
                </div>
            {/each}
        </div>
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
