import katex from 'katex';
import { marked } from 'marked';

function escapeHtml(text) {
    const element = document.createElement('div');
    element.textContent = text;
    return element.innerHTML;
}

// Raw HTML, links and images are shown as text, so model output cannot load or navigate anywhere.
marked.use({
    gfm: true,
    renderer: {
        html: ({ text }) => escapeHtml(text),
        image: ({ text }) => escapeHtml(text),
        link({ tokens }) {
            return this.parser.parseInline(tokens);
        },
    },
});

const mathPattern =
    /\$\$([\s\S]+?)\$\$|\\\[([\s\S]+?)\\\]|\\\(([\s\S]+?)\\\)|\$(?!\s)([^$\n]+?)(?<!\s)\$/g;

// Math is rendered first and kept out of Markdown, so Markdown does not change the LaTeX.
export function renderMarkdown(text) {
    const math = [];
    const source = text.replace(mathPattern, (_, display, bracket, paren, inline) => {
        const tex = display ?? bracket ?? paren ?? inline;
        const displayMode = display !== undefined || bracket !== undefined;
        math.push(katex.renderToString(tex, { displayMode, throwOnError: false }));
        return `⟦${math.length - 1}⟧`;
    });
    return marked.parse(source).replace(/⟦(\d+)⟧/g, (_, i) => math[i]);
}
