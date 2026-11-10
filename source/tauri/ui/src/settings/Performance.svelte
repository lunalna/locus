<script>
    import { invoke } from '$backend';

    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import Choice from './Choice.svelte';
    import { reload } from './reload.js';

    let settings = $state(null);
    let error = $state('');

    invoke('performance_settings')
        .then((value) => (settings = value))
        .catch((e) => (error = String(e)));

    const set = (name, value) => invoke('performance_set', { name, value }).then(reload).catch((e) => (error = String(e)));

    const rates = [
        { value: 'display', label: 'Display maximum' },
        { value: '60', label: '60 fps' },
        { value: '30', label: '30 fps' },
    ];
    const resolutions = [
        { value: 'full', label: 'Full' },
        { value: 'standard', label: 'Standard' },
    ];
</script>

<p class="mb-4 text-muted-foreground">Lower settings make Locus run smoother on slow computers and save battery. Changes reload the app and keep the open construction.</p>

{#if settings}
    <div class="flex flex-col gap-4">
        <div class="flex items-start gap-2">
            <Checkbox id="low" class="mt-0.5" checked={settings.low} onCheckedChange={(checked) => set('low', checked)} />
            <div class="flex flex-col gap-1">
                <Label for="low">Low performance mode</Label>
                <span class="text-xs text-muted-foreground">Uses 30 fps, standard resolution and no extra asymptote search, in place of the settings below.</span>
            </div>
        </div>

        <div class="flex flex-col gap-2">
            <Label for="fps">Frame rate limit</Label>
            <Choice id="fps" value={settings.low ? '30' : settings.fps} items={rates} disabled={settings.low} onchange={(value) => set('fps', value)} />
        </div>

        <div class="flex flex-col gap-2">
            <Label for="resolution">Graphics resolution</Label>
            <Choice id="resolution" value={settings.low ? 'standard' : settings.resolution} items={resolutions} disabled={settings.low} onchange={(value) => set('resolution', value)} />
            <span class="text-xs text-muted-foreground">Standard draws the graphics view with fewer pixels on high-resolution screens. It is faster, but lines look softer.</span>
        </div>

        <div class="flex items-start gap-2">
            <Checkbox id="asymptotes" class="mt-0.5" checked={settings.low ? false : settings.asymptotes} disabled={settings.low} onCheckedChange={(checked) => set('asymptotes', checked)} />
            <div class="flex flex-col gap-1">
                <Label for="asymptotes">Find asymptotes when zoomed out</Label>
                <span class="text-xs text-muted-foreground">Keeps functions like tan(x) correct at far zoom levels. Can make zooming slower for fast-changing functions.</span>
            </div>
        </div>
    </div>
{/if}

{#if error}
    <p class="mt-4 text-destructive">{error}</p>
{/if}
