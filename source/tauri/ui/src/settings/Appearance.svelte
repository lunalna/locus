<script>
    import AppSettings from './AppSettings.svelte';
    import Library from './Library.svelte';
    import { APPEARANCE } from './keys.js';

    let { exam } = $props();
</script>

<AppSettings filter={(setting) => APPEARANCE.includes(setting.id)} />

<!-- themes are Locus settings, which stay locked during an exam -->
{#if !exam}
    <h2 class="mt-6 mb-2 font-semibold">Theme</h2>
    <Library
        kind="theme"
        description="A theme changes the colors of the app and the graphics view. Files keep their original colors."
        builtin={{ name: 'Default', description: 'The light look that comes with Locus.' }}
    />
{/if}
