<script>
    import { invoke } from '$backend';

    import { Info } from '@lucide/svelte';

    import { Badge } from '$lib/components/ui/badge';
    import { Button } from '$lib/components/ui/button';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import * as Item from '$lib/components/ui/item';
    import AddItem from './AddItem.svelte';
    import { library, pick, show } from './library.svelte.js';

    let list = $state({ items: [] });
    let error = $state('');

    async function load() {
        list = await invoke('plugin_list');
        error = '';
    }

    const run = (promise) => promise.then(load).catch((e) => (error = String(e)));

    // Loads again after a plugin was added.
    $effect(() => {
        library.version;
        run(Promise.resolve());
    });
</script>

<p class="mb-4 text-muted-foreground">Plugins add panels, commands, assistant tools and windows. During exams only signed plugins made for exams run.</p>

<div class="divide-y overflow-hidden rounded-lg border">
    {#each list.items as plugin (plugin.id)}
        <Item.Root size="sm" class="cursor-pointer rounded-none hover:bg-muted">
            {#snippet child({ props })}
                <label {...props}>
                    <Item.Media class="self-center! translate-y-0!">
                        <Checkbox
                            aria-label="Enable {plugin.name ?? plugin.id}"
                            checked={plugin.enabled}
                            onCheckedChange={(enabled) => run(invoke('plugin_enable', { id: plugin.id, enabled }))}
                        />
                    </Item.Media>
                    <Item.Content>
                        <Item.Title>
                            {plugin.name ?? plugin.id}
                            {#if plugin.version}<span class="font-normal text-muted-foreground">{plugin.version}</span>{/if}
                            {#if plugin.exam}<Badge variant="outline">Exams</Badge>{:else if !plugin.signed}<Badge variant="outline">Not signed</Badge>{/if}
                            {#if !plugin.supported}<Badge variant="destructive">Needs a newer Locus</Badge>{/if}
                        </Item.Title>
                        {#if plugin.description}<Item.Description>{plugin.description}</Item.Description>{/if}
                    </Item.Content>
                    <Item.Actions>
                        <Button variant="ghost" size="icon-sm" aria-label="Details" onclick={() => show('plugin', plugin.id)}><Info /></Button>
                    </Item.Actions>
                </label>
            {/snippet}
        </Item.Root>
    {/each}
    <AddItem label="plugin" extension=".lcplugin" onclick={() => pick('plugin')} />
</div>

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
