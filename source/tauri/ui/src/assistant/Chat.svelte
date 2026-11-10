<script>
    import { tick } from 'svelte';
    import { ArrowUp, Paperclip } from '@lucide/svelte';
    import { Button } from '$lib/components/ui/button';
    import { Textarea } from '$lib/components/ui/textarea';
    import { assistant, attach, send } from '../lib/assistant.svelte.js';
    import Markdown from '../components/Markdown.svelte';

    let list = $state();
    let input = $state(null);
    let picker = $state();
    let text = $state('');

    // Keeps the newest output in view unless the student scrolled up.
    $effect.pre(() => {
        assistant.version;
        if (!list) return;
        const atBottom = list.scrollHeight - list.scrollTop - list.clientHeight < 40;
        tick().then(() => atBottom && (list.scrollTop = list.scrollHeight));
    });

    $effect(() => {
        if (assistant.focusPrompt) {
            input.focus();
            assistant.focusPrompt = false;
        }
    });

    function onsubmit(event) {
        event.preventDefault();
        if (send(text.trim())) text = '';
    }

    function onkeydown(event) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            input.form.requestSubmit();
        }
    }

    async function onpick() {
        for (const file of picker.files) {
            const image = await createImageBitmap(file).catch(() => null);
            if (image) attach(image, image.width, image.height);
        }
        picker.value = '';
        input.focus();
    }

    const count = $derived(assistant.attachments.length);
</script>

<div bind:this={list} class="flex flex-1 flex-col gap-3 overflow-auto p-4 select-text" aria-live="polite">
    {#each assistant.items as item}
        {#if item.kind === 'user'}
            <div class="max-w-[85%] self-end rounded-lg bg-muted px-3 py-2 leading-normal whitespace-pre-wrap [overflow-wrap:anywhere]">
                {#each item.images as image}
                    <img src="data:image/jpeg;base64,{image}" alt="Attached" class="mb-1.5 block max-w-[220px] rounded-md" />
                {/each}
                {item.text}
            </div>
        {:else if item.kind === 'assistant'}
            <Markdown text={item.text} class="leading-normal [overflow-wrap:anywhere]" />
        {:else if item.kind === 'thinking'}
            <details class="text-xs text-muted-foreground">
                <summary>Thinking</summary>
                <div class="mt-1.5 border-l-2 border-border pl-2.5 whitespace-pre-wrap">{item.text}</div>
            </details>
        {:else if item.kind === 'tool'}
            <div class="text-xs text-muted-foreground">{item.text}</div>
        {:else}
            <div class="text-destructive">{item.text}</div>
        {/if}
    {:else}
        <p class="m-auto max-w-[280px] text-center leading-normal text-muted-foreground">
            Ask about mathematics or Locus. The AI can also read and change your construction.
        </p>
    {/each}
    {#if assistant.busy}
        <div class="text-xs text-muted-foreground">Working...</div>
    {/if}
</div>
<form class="flex flex-col gap-2 border-t border-border p-3" {onsubmit}>
    {#if count}
        <div class="flex items-center gap-2 text-muted-foreground">
            {count} {count === 1 ? 'image' : 'images'} attached
            <Button variant="link" size="xs" onclick={() => (assistant.attachments = [])}>Remove</Button>
        </div>
    {/if}
    <div class="flex items-end gap-2">
        <input bind:this={picker} type="file" accept="image/*" multiple hidden onchange={onpick} />
        <Button variant="ghost" size="icon" aria-label="Attach images" onclick={() => picker.click()}><Paperclip /></Button>
        <Textarea
            bind:ref={input}
            bind:value={text}
            rows={1}
            placeholder="Message"
            aria-label="Message"
            class="max-h-36 min-h-9 resize-none"
            {onkeydown}
        />
        <Button type="submit" size="icon" aria-label="Send" disabled={assistant.busy}><ArrowUp /></Button>
    </div>
</form>
