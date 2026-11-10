<script>
    import { Plus } from '@lucide/svelte';

    import * as Item from '$lib/components/ui/item';

    let { label, extension, onclick } = $props();
</script>

<!-- The last row of a list, which adds an item from a file. -->
<Item.Root size="sm" class="rounded-none text-left text-muted-foreground hover:bg-muted hover:text-foreground">
    {#snippet child({ props })}
        <button type="button" {...props} {onclick}>
            <Item.Media variant="icon"><Plus /></Item.Media>
            <Item.Content>
                <Item.Title class="font-normal">Add {label}…</Item.Title>
                {#if import.meta.env.MODE !== 'web'}
                    <Item.Description>Or drop a {extension} file on an app window.</Item.Description>
                {/if}
            </Item.Content>
        </button>
    {/snippet}
</Item.Root>
