<script>
    import Setting from './Setting.svelte';

    // The settings of the app in this window, which it saves as soon as they change.
    let { filter, empty = '' } = $props();

    const bridge = window.locusAppSettings;
    let settings = $state(bridge.list().filter(filter));

    function onset(key, value) {
        const error = bridge.set(key, value);
        settings = bridge.list().filter(filter);
        return error;
    }
</script>

<div class="flex flex-col gap-4">
    {#each settings as setting (setting.key)}
        <Setting {setting} {onset} />
    {:else}
        {#if empty}<p class="text-muted-foreground">{empty}</p>{/if}
    {/each}
</div>
