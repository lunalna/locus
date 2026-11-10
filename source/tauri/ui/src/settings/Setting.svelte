<script>
    import { Button } from '$lib/components/ui/button';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { cn } from '$lib/utils.js';
    import Choice from './Choice.svelte';
    import Setting from './Setting.svelte';

    // One app setting of any type. Groups show their lead toggle and their settings, nested ones under a heading.
    let { setting, onset, top = false } = $props();

    let error = $state('');
    const set = (value) => (error = onset(setting.key, value) ?? '');
</script>

{#if setting.type === 'group' && setting.id === 'Ratio'}
    <!-- the axes ratio: two numbers and a lock without a name, which the app draws as a lock icon -->
    {@const [x, y, lock] = setting.items}
    <div class="flex flex-col gap-2">
        <Label for={x.key}>{setting.name}</Label>
        <div class="flex items-center gap-2">
            <Setting setting={{ ...x, name: '' }} {onset} />
            <span>:</span>
            <Setting setting={{ ...y, name: '' }} {onset} />
        </div>
        <Setting setting={{ ...lock, name: 'Lock ratio' }} {onset} />
    </div>
{:else if setting.type === 'group'}
    <div class="flex flex-col gap-4">
        {#if setting.lead}
            <Setting setting={setting.lead} {onset} />
        {:else if !top && setting.name}
            <h3 class="font-semibold">{setting.name}</h3>
        {/if}
        {#each setting.items as item (item.key)}
            <Setting setting={item} {onset} />
        {/each}
    </div>
{:else if setting.type === 'toggle'}
    <div class="flex items-center gap-2">
        <Checkbox id={setting.key} checked={setting.value} disabled={!setting.enabled} onCheckedChange={set} />
        <Label for={setting.key}>{setting.name}</Label>
    </div>
{:else if setting.type === 'action'}
    <div>
        <Button variant="outline" disabled={!setting.enabled} onclick={() => set(null)}>{setting.name}</Button>
    </div>
{:else}
    <div class="flex flex-1 flex-col gap-2">
        {#if setting.name}<Label for={setting.key}>{setting.name}</Label>{/if}
        {#if setting.type === 'choice'}
            <Choice
                id={setting.key}
                value={String(setting.value)}
                items={setting.values.map((label, index) => ({ value: String(index), label }))}
                disabled={!setting.enabled}
                onchange={(value) => set(Number(value))}
            />
        {:else if setting.type === 'color'}
            <div class="flex flex-wrap gap-1.5" role="radiogroup" aria-label={setting.name}>
                {#each setting.values as color, index}
                    <button
                        type="button"
                        role="radio"
                        aria-checked={index === setting.value}
                        aria-label={color || 'None'}
                        disabled={!setting.enabled}
                        class={cn('size-7 rounded-full border border-border', index === setting.value && 'ring-2 ring-primary ring-offset-2 ring-offset-background')}
                        style:background={color || 'transparent'}
                        onclick={() => set(index)}
                    ></button>
                {/each}
            </div>
        {:else if setting.type === 'range'}
            <div class="flex items-center gap-3">
                <input
                    id={setting.key}
                    type="range"
                    class="flex-1 accent-primary"
                    min={setting.min}
                    max={setting.max}
                    step={setting.step}
                    value={setting.value}
                    disabled={!setting.enabled}
                    onchange={(event) => set(Number(event.currentTarget.value))}
                />
                <span class="w-10 text-right tabular-nums">{setting.value}</span>
            </div>
        {:else if setting.type === 'text'}
            <Input
                id={setting.key}
                value={setting.value}
                disabled={!setting.enabled}
                aria-invalid={!!error}
                onchange={(event) => set(event.currentTarget.value)}
            />
        {/if}
        {#if error}<span class="text-xs text-destructive">{error}</span>{/if}
    </div>
{/if}
