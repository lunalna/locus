<script>
    import { invoke } from '$backend';

    import { ChevronDown, Info } from '@lucide/svelte';

    import { Button } from '$lib/components/ui/button';
    import * as Collapsible from '$lib/components/ui/collapsible';
    import * as Item from '$lib/components/ui/item';
    import * as RadioGroup from '$lib/components/ui/radio-group';
    import AddItem from './AddItem.svelte';
    import { library, pick, show } from './library.svelte.js';
    import { reload } from './reload.js';
    import { applyTheme } from './theme.js';

    let { kind, description, builtin } = $props();

    let list = $state({ items: [], active: null });
    let error = $state('');
    let open = $state(false);

    async function load() {
        list = await invoke('library_list', { kind });
        open ||= list.items.some((pack) => pack.more && pack.id === list.active);
        error = '';
    }

    const run = (promise) => promise.then(load).catch((e) => (error = String(e)));
    // Themes apply in place, resource packs need a reload.
    const apply = (theme) => (kind === 'theme' ? applyTheme(theme) : reload());

    // Loads again after an item was added or removed.
    $effect(() => {
        library.version;
        run(Promise.resolve());
    });

    const label = kind === 'pack' ? 'resource pack' : 'theme';
    const extension = kind === 'pack' ? '.lcpack' : '.lctheme';

    // Built-in themes come before installed ones, the less common ones are collapsed.
    let main = $derived([
        { id: null, ...builtin },
        ...list.items.filter((pack) => pack.builtin && !pack.more),
        ...list.items.filter((pack) => !pack.builtin),
    ]);
    let more = $derived(list.items.filter((pack) => pack.more));
</script>

{#snippet row(pack)}
    <Item.Root size="sm" class="cursor-pointer rounded-none hover:bg-muted">
        {#snippet child({ props })}
            <label {...props}>
                <Item.Media><RadioGroup.Item value={pack.id ?? ''} aria-label={pack.name ?? pack.id} /></Item.Media>
                <Item.Content>
                    <Item.Title>
                        {pack.name ?? pack.id}
                        {#if pack.version}<span class="font-normal text-muted-foreground">{pack.version}</span>{/if}
                    </Item.Title>
                    {#if pack.description || pack.author}<Item.Description>{pack.description ?? `By ${pack.author}`}</Item.Description>{/if}
                </Item.Content>
                <!-- The built-in entry has no details. -->
                {#if pack.id}
                    <Item.Actions>
                        <Button variant="ghost" size="icon-sm" aria-label="Details" onclick={() => show(kind, pack.id)}><Info /></Button>
                    </Item.Actions>
                {/if}
            </label>
        {/snippet}
    </Item.Root>
{/snippet}

<p class="mb-4 text-muted-foreground">{description}</p>

<!-- The built-in entry has no id, so it uses the empty string as its radio value. -->
<RadioGroup.Root
    value={list.active ?? ''}
    onValueChange={(id) => run(invoke('library_activate', { kind, id: id || null }).then(apply))}
    class="gap-0 divide-y overflow-hidden rounded-lg border"
>
    {#each main as pack (pack.id)}
        {@render row(pack)}
    {/each}
    {#if more.length}
        <!-- The content comes first so the trigger stays above the add row. -->
        <Collapsible.Root bind:open class="divide-y">
            <Collapsible.Content class="divide-y">
                {#each more as pack (pack.id)}
                    {@render row(pack)}
                {/each}
            </Collapsible.Content>
            <Collapsible.Trigger class="flex w-full items-center gap-2.5 px-3 py-2.5 text-left text-muted-foreground hover:bg-muted hover:text-foreground">
                <ChevronDown class="size-4 transition-transform {open ? 'rotate-180' : ''}" />
                {open ? 'Fewer themes' : `${more.length} more themes`}
            </Collapsible.Trigger>
        </Collapsible.Root>
    {/if}
    <AddItem {label} {extension} onclick={() => pick(kind)} />
</RadioGroup.Root>

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
