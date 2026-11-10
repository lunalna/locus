import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const path = (file) => fileURLToPath(new URL(file, import.meta.url));
const run = (command, cwd) => JSON.parse(execSync(command, { cwd: path(cwd), maxBuffer: 1 << 28 }));

const logo = (name) => `data:image/svg+xml,${encodeURIComponent(readFileSync(path(`../icons/wordmarks/${name}.svg`), 'utf8'))}`;

// Provides the version, the wordmark and the licenses of all npm packages and Rust crates as virtual:about.
export default (mode) => ({
    name: 'about',
    resolveId: (id) => (id === 'virtual:about' ? '\0virtual:about' : null),
    load(id) {
        if (id !== '\0virtual:about') return null;
        const packages = Object.values(run('pnpm licenses list --json', '.'))
            .flat()
            .map((item) => ({
                name: item.name,
                version: item.versions.join(', '),
                license: item.license,
                url: item.homepage ?? `https://www.npmjs.com/package/${item.name}`,
            }));
        // The web version has no Rust code.
        const crates = mode === 'web' ? [] : run('cargo metadata --format-version 1', '..').packages;
        const licenses = crates
            .filter((item) => item.source)
            .map((item) => ({
                name: item.name,
                version: item.version,
                license: item.license,
                url: item.repository ?? `https://crates.io/crates/${item.name}`,
            }));
        return `export default ${JSON.stringify({
            version: JSON.parse(readFileSync(path('../tauri.conf.json'))).version,
            logo: logo('locus'),
            logoDark: logo('locus-dark'),
            licenses: [...packages, ...licenses],
        })}`;
    },
});
