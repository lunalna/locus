<script>
    import Setting from './Setting.svelte';

    // A settings tab of the app, like Graphics: one of its groups, or its settings outside groups when group is null.
    let { tab, group = null } = $props();

    const bridge = window.locusAppSettings;
    let items = $state(load());

    function load() {
        const all = bridge.tabs().find((entry) => entry.id === tab)?.items ?? [];
        return group ? all.filter((item) => item.key === group) : all.filter((item) => item.type !== 'group');
    }

    function onset(key, value) {
        const error = bridge.set(key, value);
        items = load();
        return error;
    }
</script>

<div class="flex flex-col gap-4">
    {#each items as item (item.key)}
        <Setting setting={item} {onset} top />
    {/each}
</div>
