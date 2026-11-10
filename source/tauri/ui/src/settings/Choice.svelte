<script>
    import * as Select from '$lib/components/ui/select';

    let { id, value, items, disabled = false, onchange } = $props();
</script>

<Select.Root type="single" {value} {items} {disabled} onValueChange={onchange}>
    <Select.Trigger {id} class="w-full">
        <Select.Value />
    </Select.Trigger>
    <Select.Content>
        {#each items as item (item.value)}
            <Select.Item value={item.value} label={item.label} />
        {/each}
    </Select.Content>
</Select.Root>
