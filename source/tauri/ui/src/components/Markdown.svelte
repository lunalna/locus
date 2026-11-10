<script>
    import { renderMarkdown } from '../lib/markdown.js';

    let { text, class: className = '' } = $props();
</script>

<div class="markdown {className}">{@html renderMarkdown(text)}</div>
