#!/usr/bin/env bash
# Development and release commands for the Locus desktop and iPad apps.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
TAURI="$ROOT/source/tauri"
DIST="$ROOT/dist"
NAME="Locus"
ID=xyz.lunalna.locus
# Tauri only lists simulators when no physical device is connected, so one is named by default.
SIMULATOR="iPad Pro 13-inch (M5)"

# --host binds the dev servers to all interfaces, so other devices can reach it.
BIND=localhost
args=()
for arg in "$@"; do
    if [[ "$arg" == --host ]]; then BIND=0.0.0.0; else args+=("$arg"); fi
done
set -- ${args[@]+"${args[@]}"}

gradle() { (cd "$ROOT/source/web" && "$ROOT/gradlew" "$@"); }
tauri() { (cd "$TAURI" && cargo tauri "$@"); }

web() {
    (cd "$TAURI/ui" && pnpm install --frozen-lockfile && pnpm build)
    gradle :web:desktop -Pgmodule=org.geogebra.web.SuperWeb
}

# The web version: static files in dist/web that any web server can host.
webapp() {
    (cd "$TAURI/ui" && pnpm install --frozen-lockfile && pnpm build --mode web)
    gradle :web:webApp -Pgmodule=org.geogebra.web.SuperWeb
    rm -rf "$DIST/web"
    mkdir -p "$DIST"
    cp -R "$ROOT/source/web/web/build/webapp" "$DIST/web"
    echo "$DIST/web"
}

# Signs the update files the desktop apps download.
export TAURI_SIGNING_PRIVATE_KEY="${TAURI_SIGNING_PRIVATE_KEY:-$HOME/.locus/updater.key}"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

# Signing and notarization use APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD and APPLE_TEAM_ID when set.
macos() {
    local bundle="$TAURI/target/universal-apple-darwin/release/bundle/macos"
    # actool crashes during bundling when it inherits stdin
    tauri build --target universal-apple-darwin --bundles app < /dev/null
    mkdir -p "$DIST"
    rm -f "$DIST/$NAME-macos.zip"
    ditto -c -k --keepParent "$bundle/$NAME.app" "$DIST/$NAME-macos.zip"
    cp "$bundle/$NAME.app.tar.gz" "$DIST/$NAME-macos.app.tar.gz"
    cp "$bundle/$NAME.app.tar.gz.sig" "$DIST/$NAME-macos.app.tar.gz.sig"
    echo "$DIST/$NAME-macos.zip"
}

# Background jobs ignore Ctrl+C, so everything started here is stopped on exit.
gwt_server() {
    # An already running server (e.g. from ./dev.sh gwt) is reused so it keeps its compile cache.
    curl -s -o /dev/null "http://localhost:8888/classic-desktop.html" && return
    trap 'trap - EXIT INT TERM; kill 0' EXIT INT TERM
    mkdir -p "$ROOT/build"
    gradle --no-daemon --console=plain :web:gwtDevMode -Pgbind="$BIND" > "$ROOT/build/gwt-dev.log" 2>&1 &
    echo "GWT dev server log: build/gwt-dev.log"
    until curl -s -o /dev/null "http://localhost:8888/classic-desktop.html"; do sleep 2; done
}

# gen/ is not committed, so a fresh checkout creates the Xcode project first.
# The project holds the bundle identifier, so a changed identifier recreates it.
ios_project() {
    if ! grep -qs "PRODUCT_BUNDLE_IDENTIFIER: $ID$" "$TAURI/gen/apple/project.yml"; then
        rm -rf "$TAURI/gen/apple"
        tauri ios init --ci --skip-targets-install
    fi
    cp "$TAURI"/icons/ios/*.png "$TAURI/gen/apple/Assets.xcassets/AppIcon.appiconset/"
    # Tauri fails to replace an existing app bundle, so old build output is removed.
    rm -rf "$TAURI/gen/apple/build"
}

windows() {
    local target=x86_64-pc-windows-msvc
    # .cargo/config.toml already links the CRT statically; Tauri's vcruntime override conflicts with it
    export STATIC_VCRUNTIME=false
    if [[ "$(uname)" == MINGW* || "$(uname)" == MSYS* ]]; then
        tauri build --target "$target" --bundles nsis
    else
        # cross-compiling needs llvm-rc and clang-cl from Homebrew's llvm, and makensis from Homebrew's nsis
        export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
        tauri build --runner cargo-xwin --target "$target" --bundles nsis
    fi
    local setup="$TAURI/target/$target/release/bundle/nsis/${NAME}_$(version)_x64-setup.exe"
    mkdir -p "$DIST"
    cp "$setup" "$DIST/$NAME-windows-setup.exe"
    cp "$setup.sig" "$DIST/$NAME-windows-setup.exe.sig"
    echo "$DIST/$NAME-windows-setup.exe"
}

version() { jq -j .version "$TAURI/tauri.conf.json"; }

# Uploads the release builds in dist/ to GitHub, with the latest.json the desktop apps check for updates.
publish() {
    local repo=lunalna/locus v
    v=$(version)
    local url="https://github.com/$repo/releases/download/v$v"
    jq -n --arg v "$v" --arg url "$url" --arg date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --arg mac "$(cat "$DIST/$NAME-macos.app.tar.gz.sig")" \
        --arg win "$(cat "$DIST/$NAME-windows-setup.exe.sig")" '
        {signature: $mac, url: "\($url)/Locus-macos.app.tar.gz"} as $macos |
        {version: $v, pub_date: $date, platforms: {
            "darwin-aarch64": $macos,
            "darwin-x86_64": $macos,
            "windows-x86_64": {signature: $win, url: "\($url)/Locus-windows-setup.exe"}
        }}' > "$DIST/latest.json"
    gh release create "v$v" --repo "$repo" --title "$NAME $v" --notes "$(git tag -l --format="%(contents:body)" "v$v")" \
        "$DIST/$NAME-macos.zip" "$DIST/$NAME-macos.app.tar.gz" \
        "$DIST/$NAME-windows-setup.exe" "$DIST/latest.json"
}

# Builds a resource pack from the original assets of a GeoGebra source checkout.
official_pack() {
    local src="$1" out="$DIST/official-pack"
    rm -rf "$out"
    mkdir -p "$out/lang"
    for resources in "$src"/source/shared/common/src/main/resources "$src"/source/web/*/src/main/resources; do
        rsync -a --include='*/' --include='*.svg' --exclude='*' --prune-empty-dirs "$resources/" "$out/assets/"
    done
    cp "$src"/source/shared/common-jre/src/main/resources/org/geogebra/common/jre/properties/{menu,command,symbols}*.properties "$out/lang/"
    cp "$src/source/tauri/icons/icon.png" "$out/icon.png"
    cat > "$out/pack.json" <<EOF
{
  "format": 1,
  "id": "official-assets",
  "name": "Official GeoGebra assets",
  "version": "1.0.0",
  "author": "GeoGebra GmbH",
  "license": "CC BY-NC-SA 4.0",
  "productName": "GeoGebra",
  "icon": "icon.png",
  "colors": {
    "primary-100": "#F3F0FF",
    "primary-200": "#D2CCFF",
    "primary-300": "#B9B1FC",
    "primary-400": "#A397FC",
    "primary-500": "#8172F7",
    "primary-600": "#6557D2",
    "primary-700": "#5145A8",
    "primary-800": "#3B337A",
    "primary-900": "#191533",
    "highlight": "#6161FF",
    "highlight-light": "#CCCCFF"
  },
  "urls": {
    "manual": "https://geogebra.github.io/docs/manual/",
    "tutorials": "https://www.geogebra.org/m/",
    "forum": "https://help.geogebra.org/",
    "reportBug": "https://www.reddit.com/r/geogebra/",
    "privacy": "https://www.geogebra.org/privacy",
    "license": "https://www.geogebra.org/license",
    "website": "https://www.geogebra.org/",
    "materialsApi": "https://api.geogebra.org/v1.0",
    "loginApi": "https://accounts.geogebra.org/api/index.php",
    "deployScript": "https://www.geogebra.org/apps/deployggb.js"
  }
}
EOF
    rm -f "$out.lcpack"
    (cd "$out" && zip -qr "$out.lcpack" .)
    echo "$out.lcpack"
}

case "${1:-}" in
    gwt)
        gradle :web:gwtDevMode -Pgbind="$BIND"
        ;;
    app)
        tauri dev
        ;;
    dev)
        gwt_server
        tauri dev
        ;;
    web-dev)
        gwt_server
        (cd "$TAURI/ui" && pnpm exec vite --mode web --host "$BIND")
        ;;
    ios)
        web
        ios_project
        tauri ios run "${2:-$SIMULATOR}"
        ;;
    ios-device)
        web
        ios_project
        # `tauri ios run` loses its export options before installing on a device, so build and install here
        tauri ios build
        xcrun devicectl device install app --device "$2" \
            "$TAURI/gen/apple/build/locus_iOS.xcarchive/Products/Applications/$NAME.app"
        xcrun devicectl device process launch --device "$2" "$ID"
        ;;
    ios-dev)
        gwt_server
        ios_project
        tauri ios dev "${2:-$SIMULATOR}"
        ;;
    html)
        gradle :web:copyDevHtml
        ;;
    web)
        web
        ;;
    webapp)
        webapp
        ;;
    macos)
        web
        macos
        ;;
    windows)
        web
        windows
        ;;
    official-pack)
        official_pack "$2"
        ;;
    release)
        web
        macos
        windows
        ;;
    publish)
        publish
        ;;
    *)
        cat <<EOF
Usage: ./dev.sh <command>

  dev       GWT dev server and desktop app (with the Vite dev server) together
  gwt       GWT dev server on localhost:8888
  web-dev   GWT dev server and the web version on localhost:5174, with the settings and assistant from the sources
  app       Desktop app and Vite dev server against the running GWT dev server
  html      Regenerate the dev server's HTML pages
  web       Production web build for the desktop app
  webapp    Web version in dist/web, to host on a web server
  macos     Universal macOS app in dist/
  windows   Windows installer in dist/
  ios       Production build on a simulator (default "$SIMULATOR"), e.g. ./dev.sh ios "iPad Air 11-inch (M4)"
  ios-device Production build on a connected iPad, e.g. ./dev.sh ios-device "iPad AR"
  ios-dev   GWT dev server and simulator app (with the Vite dev server) together
  release   macOS app and Windows installer
  publish   Upload the release in dist/ to GitHub, so the apps update to it
  official-pack Resource pack (.lcpack) with the original assets of a GeoGebra checkout, e.g. ./dev.sh official-pack ~/geogebra

  --host    With gwt, dev, web-dev or ios-dev: bind the dev servers to all interfaces, e.g. ./dev.sh gwt --host
EOF
        exit 1
        ;;
esac
