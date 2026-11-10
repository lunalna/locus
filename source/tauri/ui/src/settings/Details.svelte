<script>
    import { invoke } from '$backend';
    import { AppWindow, Bot, Bug, Code, Eye, FileText, Globe, GraduationCap, House, PanelRight, PencilRuler, Wifi, Wrench } from '@lucide/svelte';

    import { Badge } from '$lib/components/ui/badge';
    import { Button } from '$lib/components/ui/button';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import * as Dialog from '$lib/components/ui/dialog';
    import { Input } from '$lib/components/ui/input';
    import * as Item from '$lib/components/ui/item';
    import { Label } from '$lib/components/ui/label';
    import Choice from './Choice.svelte';
    import { add, close, library, remove, setSetting } from './library.svelte.js';

    const kinds = { plugin: 'Plugin', pack: 'Resource pack', theme: 'Theme' };
    const links = { source: ['Source code', Code], website: ['Website', House], issues: ['Issue tracker', Bug] };
    const permissionNames = {
        'construction:read': ['Read the construction in the app windows', Eye],
        'construction:write': ['Change the construction', PencilRuler],
        files: ['Open and save files you choose', FileText],
        websites: ['Show websites in its window', Globe],
        assistant: ['Use AI features and allow them during exams', Bot],
        exam: ['Allow leaving the window during exams and add to the exam log', GraduationCap],
    };

    // The dialog keeps showing the last item while it closes.
    let item = $state.raw(null);
    $effect.pre(() => {
        if (library.details) item = library.details;
    });
    const open = $derived(Boolean(library.details || library.error));
    const plugin = $derived(item?.plugin);
    const permissions = $derived(
        plugin
            ? [
                  ...(plugin.window ? [['Show a window above other apps', AppWindow]] : []),
                  ...(plugin.panel ? [['Show a panel in the app windows', PanelRight]] : []),
                  ...(plugin.permissions ?? []).map((name) => permissionNames[name] ?? [name, Eye]),
                  ...(plugin.network?.length ? [[`Connect to ${plugin.network.join(', ')}`, Wifi]] : []),
                  ...(plugin.tools.length ? [[`Give the AI tools: ${plugin.tools.join(', ')}`, Wrench]] : []),
              ]
            : [],
    );
    const byline = $derived(
        item && [item.builtin ? 'Comes with Locus' : item.author && `By ${item.author}`, item.license].filter(Boolean).join(' · '),
    );
</script>

<Dialog.Root {open} onOpenChange={(value) => !value && close()}>
    <Dialog.Content class="max-h-[calc(100vh-4rem)] overflow-auto sm:max-w-md">
        {#if library.error}
            <Dialog.Header>
                <Dialog.Title>This file cannot be added</Dialog.Title>
                <Dialog.Description>{library.error}</Dialog.Description>
            </Dialog.Header>
            <Dialog.Footer>
                <Button onclick={close}>Close</Button>
            </Dialog.Footer>
        {:else if item}
            <Dialog.Header>
                <Dialog.Title>{library.adding ? `Add ${item.name ?? item.id}?` : (item.name ?? item.id)}</Dialog.Title>
                {#if byline}<Dialog.Description>{byline}</Dialog.Description>{/if}
            </Dialog.Header>

            <div class="flex flex-wrap gap-1.5">
                {#if item.version}<Badge variant="outline">Version {item.version}</Badge>{/if}
                {#if plugin}
                    {#if plugin.exam}
                        <Badge variant="outline">Runs during exams</Badge>
                    {:else}
                        <Badge variant="outline">{plugin.signed ? 'Not made for exams' : 'Not signed'}</Badge>
                    {/if}
                {/if}
                {#if item.active}<Badge>Active</Badge>{/if}
            </div>

            {#if item.description}<p>{item.description}</p>{/if}

            {#if plugin}
                <div class="flex flex-col gap-1.5">
                    <h3 class="font-medium">This plugin can</h3>
                    <div class="divide-y rounded-lg border">
                        {#each permissions as [text, Icon]}
                            <Item.Root size="xs" class="rounded-none">
                                <Item.Media variant="icon" class="text-muted-foreground"><Icon /></Item.Media>
                                <Item.Content><Item.Title class="line-clamp-none font-normal">{text}</Item.Title></Item.Content>
                            </Item.Root>
                        {:else}
                            <Item.Root size="xs" class="rounded-none">
                                <Item.Content><Item.Title class="line-clamp-none font-normal">Run in the background without access to Locus</Item.Title></Item.Content>
                            </Item.Root>
                        {/each}
                    </div>
                </div>

                {#if !library.adding && plugin.settings?.length}
                    <div class="flex flex-col gap-3">
                        <h3 class="font-medium">Settings</h3>
                        {#each plugin.settings as setting (setting.key)}
                            {@const id = `setting-${setting.key}`}
                            {#if setting.type === 'toggle'}
                                <div class="flex items-center gap-2">
                                    <Checkbox {id} checked={plugin.values[setting.key] === true} onCheckedChange={(value) => setSetting(setting.key, value)} />
                                    <Label for={id}>{setting.label}</Label>
                                </div>
                            {:else}
                                <div class="flex flex-col gap-1.5">
                                    <Label for={id}>{setting.label}</Label>
                                    {#if setting.type === 'choice'}
                                        <Choice {id} value={plugin.values[setting.key]} items={setting.options} onchange={(value) => setSetting(setting.key, value)} />
                                    {:else}
                                        <Input {id} value={plugin.values[setting.key] ?? ''} onchange={(event) => setSetting(setting.key, event.target.value)} />
                                    {/if}
                                </div>
                            {/if}
                        {/each}
                    </div>
                {/if}
            {/if}

            {#if Object.keys(item.links ?? {}).length}
                <div class="flex flex-wrap gap-2">
                    {#each Object.entries(item.links) as [name, url]}
                        {@const [label, Icon] = links[name] ?? [name, Globe]}
                        <Button variant="outline" size="sm" title={url} onclick={() => invoke('library_link', { url })}>
                            <Icon />{label}
                        </Button>
                    {/each}
                </div>
            {/if}

            {#if library.adding && item.installed}
                <p class="text-muted-foreground">This replaces the installed {kinds[item.kind].toLowerCase()} with the same id.</p>
            {/if}

            <Dialog.Footer>
                {#if library.adding}
                    <Button variant="outline" onclick={close}>Cancel</Button>
                    <Button onclick={add}>Add</Button>
                {:else}
                    {#if !item.builtin}<Button variant="destructive" class="sm:mr-auto" onclick={remove}>Remove</Button>{/if}
                    <Button onclick={close}>Close</Button>
                {/if}
            </Dialog.Footer>
        {/if}
    </Dialog.Content>
</Dialog.Root>
