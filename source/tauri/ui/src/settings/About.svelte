<script>
    import { invoke } from '$backend';
    import { X } from '@lucide/svelte';
    import about from 'virtual:about';

    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';
    import * as Tabs from '$lib/components/ui/tabs';
    import bundled from './licenses.js';

    let { modal } = $props();

    const licenses = [...bundled, ...about.licenses].sort((a, b) => a.name.localeCompare(b.name));
    let search = $state('');
    const shown = $derived(licenses.filter((item) => `${item.name} ${item.license}`.toLowerCase().includes(search.toLowerCase())));
    // Links stay closed during an exam.
    const exam = $derived(modal.about && window.locusAppSettings.exam());
    const open = (url) => invoke('library_link', { url });
    // Themes with a dark background get the light wordmark, like the app's own logo.
    const logo = $derived.by(() => {
        const bg = parseInt(((modal.about && window.locusTheme) || { background: '#ffffff' }).background.slice(1), 16);
        return (bg >> 16) * 299 + ((bg >> 8) & 255) * 587 + (bg & 255) * 114 < 128000 ? about.logoDark : about.logo;
    });
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && (modal.about = false)} />

{#if modal.about}
    <div
        class="fixed inset-0 flex items-center justify-center bg-black/30 font-sans text-[14px] text-foreground"
        role="presentation"
        onclick={(event) => event.target === event.currentTarget && (modal.about = false)}
    >
        <div class="flex max-h-[calc(100vh-32px)] w-[640px] max-w-[calc(100vw-32px)] flex-col rounded-xl bg-background shadow-xl" role="dialog" aria-label="About Locus">
            <header class="flex items-center border-b border-border px-5 py-3">
                <h1 class="flex-1 text-[17px] font-semibold">About Locus</h1>
                <Button variant="ghost" size="icon-sm" aria-label="Close" onclick={() => (modal.about = false)}><X /></Button>
            </header>
            <Tabs.Root value="about" class="min-h-0 flex-1 gap-4 p-5">
                <Tabs.List>
                    <Tabs.Trigger value="about">About</Tabs.Trigger>
                    <Tabs.Trigger value="licenses">Open source licenses</Tabs.Trigger>
                </Tabs.List>
                <Tabs.Content value="about" class="flex flex-col gap-6 overflow-auto text-[14px]">
                    <div class="flex flex-col items-start gap-2">
                        <img src={logo} alt="Locus" class="h-12" />
                        <p class="text-muted-foreground">Version {about.version}</p>
                        <p>A fast math app for graphing, geometry, 3D and CAS.</p>
                        <Button variant="outline" size="sm" disabled={exam} onclick={() => open('https://locusmath.app')}>locusmath.app</Button>
                    </div>
                    <div class="flex flex-col gap-2">
                        <h2 class="font-semibold">GeoGebra</h2>
                        <p>
                            Locus is originally forked from GeoGebra. Made with GeoGebra®. Locus is not affiliated with GeoGebra and is not an official GeoGebra
                            product.
                        </p>
                        <p class="text-xs text-muted-foreground">
                            The GeoGebra source code is used under the EUPL 1.2. The GeoGebra language files and style sheets are used under CC BY-NC-SA 4.0.
                        </p>
                        <div class="flex gap-2">
                            <Button variant="outline" size="sm" disabled={exam} onclick={() => open('https://www.geogebra.org')}>www.geogebra.org</Button>
                            <Button variant="outline" size="sm" disabled={exam} onclick={() => open('https://www.geogebra.org/license')}>
                                www.geogebra.org/license
                            </Button>
                        </div>
                    </div>
                </Tabs.Content>
                <Tabs.Content value="licenses" class="flex min-h-0 flex-col gap-2 text-[14px]">
                    <Input bind:value={search} placeholder="Search" />
                    <div class="h-[440px] min-h-0 overflow-auto">
                        {#each shown as item}
                            <div class="flex items-baseline gap-2 border-b border-border py-2">
                                <button type="button" class="shrink-0 font-medium enabled:hover:underline" disabled={exam} onclick={() => open(item.url)}>
                                    {item.name}
                                </button>
                                <span class="shrink-0 text-xs text-muted-foreground">{item.version}</span>
                                <span class="ml-auto truncate text-xs text-muted-foreground" title={item.license}>{item.license}</span>
                            </div>
                        {:else}
                            <p class="py-2 text-muted-foreground">Nothing found.</p>
                        {/each}
                    </div>
                </Tabs.Content>
            </Tabs.Root>
        </div>
    </div>
{/if}
