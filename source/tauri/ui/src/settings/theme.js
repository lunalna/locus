// Sets the active theme's colors as --locus-* variables, which also reach the settings' shadow root.
export function paint() {
    const style = document.documentElement.style;
    for (const name of [...style].filter((name) => name.startsWith('--locus-'))) style.removeProperty(name);
    const colors = { ...window.resourcePackColors, ...window.locusTheme };
    for (const [name, value] of Object.entries(colors)) style.setProperty(`--locus-${name}`, value);
}

// Applies a theme returned by library_activate without reloading the page.
export function applyTheme({ colors, theme, css }) {
    window.resourcePackColors = colors;
    if (theme) window.locusTheme = theme;
    else delete window.locusTheme;
    paint();
    let link = document.getElementById('locus_theme');
    if (css) {
        window.locusThemeCss = css;
        if (!link) {
            link = Object.assign(document.createElement('link'), { id: 'locus_theme', rel: 'stylesheet' });
            document.head.append(link);
        }
        // the URL stays the same between themes, so it changes to load the new file
        link.href = `${css}?${Date.now()}`;
    } else {
        delete window.locusThemeCss;
        link?.remove();
    }
    if (!window.resourcePackIcon) window.locusLogo?.();
    window.locusApplyTheme?.();
}
