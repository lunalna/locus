<script>
    import { invoke } from '$backend';

    import { Button } from '$lib/components/ui/button';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import AppSettings from './AppSettings.svelte';
    import Choice from './Choice.svelte';
    import { GENERAL } from './keys.js';
    import { reload } from './reload.js';

    // a browser has no windows to restore or close
    const desktop = import.meta.env.MODE !== 'web';

    let settings = $state(null);
    let error = $state('');
    let version = $state(0);

    if (desktop) {
        invoke('general_settings')
            .then((value) => (settings = value))
            .catch((e) => (error = String(e)));
    }

    function set(name, value) {
        settings[name] = value;
        invoke('general_set', { name, value }).catch((e) => (error = String(e)));
    }

    // these are read by the app when the page loads
    function setAndReload(name, value) {
        invoke('general_set', { name, value }).then(reload).catch((e) => (error = String(e)));
    }

    function reset() {
        window.locusAppSettings.reset();
        version++;
    }

    const startups = [
        { value: 'last', label: 'Windows from last time' },
        { value: 'classic', label: 'Locus Classic' },
        { value: 'suite', label: 'Locus Suite' },
        { value: 'graphing', label: 'Locus Graphing' },
        { value: 'geometry', label: 'Locus Geometry' },
        { value: '3d', label: 'Locus 3D' },
        { value: 'cas', label: 'Locus CAS' },
        { value: 'scientific', label: 'Locus Scientific' },
        { value: 'notes', label: 'Locus Whiteboard' },
    ];
</script>

<div class="flex flex-col gap-4">
    {#key version}
        <AppSettings filter={(setting) => GENERAL.includes(setting.id)} />
    {/key}

    {#if settings}
        <div class="flex flex-col gap-2">
            <Label for="startup">Open on launch</Label>
            <Choice id="startup" value={settings.startup} items={startups} onchange={(value) => set('startup', value)} />
        </div>

        <div class="flex items-start gap-2">
            <Checkbox id="confirm" class="mt-0.5" checked={settings.confirm_close} onCheckedChange={(checked) => set('confirm_close', checked)} />
            <div class="flex flex-col gap-1">
                <Label for="confirm">Ask to save before closing a window</Label>
                <span class="text-xs text-muted-foreground">Only when the window has unsaved changes.</span>
            </div>
        </div>

        <div class="flex items-start gap-2">
            <Checkbox id="classic-settings" class="mt-0.5" checked={settings.classic_settings} onCheckedChange={(checked) => setAndReload('classic_settings', checked)} />
            <div class="flex flex-col gap-1">
                <Label for="classic-settings">Use the classic settings panel</Label>
                <span class="text-xs text-muted-foreground">Graphics, Algebra and other app settings open in the side panel. Settings in the main menu still opens these settings.</span>
            </div>
        </div>

        <div class="flex items-start gap-2">
            <Checkbox id="hide-app-switcher" class="mt-0.5" checked={settings.hide_app_switcher} onCheckedChange={(checked) => setAndReload('hide_app_switcher', checked)} />
            <Label for="hide-app-switcher">Hide the app switcher in Locus Classic</Label>
        </div>
    {/if}

    <div class="flex flex-col items-start gap-2">
        <Button variant="outline" onclick={reset}>Reset app settings</Button>
        <span class="text-xs text-muted-foreground">Language, Math and Appearance settings are shared by all apps. This resets them, along with the view settings of the app in this window.</span>
    </div>
</div>

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
