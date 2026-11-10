<script>
    import { Blocks, Bot, Box, ChartSpline, Gauge, Package, Palette, Settings2, Sheet, Sigma, SlidersHorizontal, SquareFunction, X } from '@lucide/svelte';
    import { BitsConfig } from 'bits-ui';

    import { Button } from '$lib/components/ui/button';
    import { cn } from '$lib/utils.js';
    import About from './About.svelte';
    import AppSettings from './AppSettings.svelte';
    import Appearance from './Appearance.svelte';
    import AiGeneral from './AiGeneral.svelte';
    import AiProviders from './AiProviders.svelte';
    import AiSidebar from './AiSidebar.svelte';
    import Details from './Details.svelte';
    import General from './General.svelte';
    import { APPEARANCE, GENERAL } from './keys.js';
    import Library from './Library.svelte';
    import Mcp from './Mcp.svelte';
    import Performance from './Performance.svelte';
    import Plugins from './Plugins.svelte';
    import ViewSettings from './ViewSettings.svelte';

    let { modal, container } = $props();

    const tabIcons = { DrawingPad: ChartSpline, DrawingPad2: ChartSpline, GraphicsView3D: Box, Spreadsheet: Sheet, CAS: SquareFunction };

    // The app's own settings tabs, like Graphics, with their groups as subsections.
    function tabSections() {
        return window.locusAppSettings.tabs().flatMap((tab) => {
            const groups = tab.items.filter((item) => item.type === 'group');
            const loose = tab.items.length > groups.length;
            if (!loose && !groups.length) return [];
            return {
                name: tab.name,
                ids: [tab.id],
                icon: tabIcons[tab.id] ?? SlidersHorizontal,
                component: loose ? ViewSettings : null,
                props: { tab: tab.id },
                children: groups.map((group) => ({ name: group.name, component: ViewSettings, props: { tab: tab.id, group: group.key } })),
            };
        });
    }

    const web = import.meta.env.MODE === 'web';

    function build() {
        const exam = window.locusAppSettings.exam();
        const app = [
            { name: 'General', ids: ['General', 'Advanced'], icon: Settings2, component: General, props: {} },
            {
                name: 'Math',
                ids: ['Algebra'],
                icon: Sigma,
                component: AppSettings,
                props: {
                    filter: (setting) => !GENERAL.includes(setting.id) && !APPEARANCE.includes(setting.id),
                    empty: 'This app has no math settings.',
                },
            },
            { name: 'Appearance', icon: Palette, component: Appearance, props: { exam } },
            ...tabSections(),
        ];
        // only the app's own settings can change during an exam
        if (exam) return app;
        return [
            ...app,
            {
                name: 'Resource packs',
                icon: Package,
                component: Library,
                props: {
                    kind: 'pack',
                    description: 'A resource pack replaces icons, translations, styles and links. Only one pack can be active.',
                    builtin: { name: 'Built-in', description: 'The resources that come with Locus.' },
                },
            },
            { name: 'Performance', icon: Gauge, component: Performance, props: {} },
            {
                name: 'AI features',
                icon: Bot,
                component: null,
                children: [
                    { name: 'General', component: AiGeneral, props: {} },
                    { name: 'Model providers', component: AiProviders, props: {} },
                    { name: 'AI sidebar', component: AiSidebar, props: {} },
                    // a browser cannot run an MCP server
                    ...(web ? [] : [{ name: 'MCP server', component: Mcp, props: {} }]),
                ],
            },
            // a browser cannot run plugin windows
            ...(web ? [] : [{ name: 'Plugins', icon: Blocks, component: Plugins, props: {} }]),
        ];
    }

    // A section without a page of its own shows its first subsection.
    const page = (section) => (section.component ? section : section.children[0]);

    let sections = $state.raw([]);
    let selected = $state.raw(null);

    // Built on opening, since the app may have changed its views and the exam state since.
    $effect.pre(() => {
        if (!modal.open) return;
        const built = build();
        sections = built;
        selected = page(built.find((section) => section.ids?.includes(modal.section)) ?? built[0]);
    });

    const inside = (section) => section === selected || section.children?.includes(selected);
    const itemClass = (active) => cn('h-9 w-full justify-start gap-2.5 px-3 text-[14px] font-normal', active && 'bg-primary/15 font-semibold text-primary hover:bg-primary/15 hover:text-primary');
</script>

<!-- An open popup handles Escape first and marks it handled. -->
<svelte:window onkeydown={(event) => event.key === 'Escape' && !event.defaultPrevented && (modal.open = false)} />

<!-- Popups render inside the shadow root so they get its styles. -->
<BitsConfig defaultPortalTo={container}>
    {#if modal.open}
        <div
            class="fixed inset-0 flex items-center justify-center bg-black/30 font-sans text-[14px] text-foreground"
            role="presentation"
            onclick={(event) => event.target === event.currentTarget && (modal.open = false)}
        >
            <div class="flex h-[640px] max-h-[calc(100vh-32px)] w-[920px] max-w-[calc(100vw-32px)] flex-col rounded-xl bg-background shadow-xl" role="dialog" aria-label="Locus settings">
                <header class="flex items-center border-b border-border px-5 py-3">
                    <h1 class="flex-1 text-[17px] font-semibold">Locus settings</h1>
                    <Button variant="ghost" size="icon-sm" aria-label="Close" onclick={() => (modal.open = false)}><X /></Button>
                </header>
                <div class="flex min-h-0 flex-1">
                    <nav class="flex w-[220px] flex-col gap-0.5 overflow-auto border-r border-border p-2">
                        {#each sections as section}
                            <Button
                                variant="ghost"
                                class={itemClass(section === selected)}
                                aria-current={section === selected ? 'page' : undefined}
                                onclick={() => (selected = page(section))}
                            >
                                <section.icon class="size-[18px]" />
                                {section.name}
                            </Button>
                            {#if section.children?.length && inside(section)}
                                <div class="ml-[21px] flex flex-col gap-0.5 border-l border-border pl-2">
                                    {#each section.children as child}
                                        <Button
                                            variant="ghost"
                                            class={itemClass(child === selected)}
                                            aria-current={child === selected ? 'page' : undefined}
                                            onclick={() => (selected = child)}
                                        >
                                            {child.name}
                                        </Button>
                                    {/each}
                                </div>
                            {/if}
                        {/each}
                    </nav>
                    <section class="min-w-0 flex-1 overflow-auto p-5">
                        {#key selected}
                            {#if selected}<selected.component {...selected.props} />{/if}
                        {/key}
                    </section>
                </div>
            </div>
        </div>
    {/if}
    <Details />
    <About {modal} />
</BitsConfig>
