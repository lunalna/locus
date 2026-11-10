# Locus

Locus is a desktop and iPad app for dynamic mathematics: graphing, geometry, 3D, CAS, a scientific calculator and a whiteboard. It runs on macOS, Windows and iPadOS.

## Features

- Increased performance
- Apps: Classic, Suite, Graphing, Geometry, 3D, CAS, Scientific and Whiteboard
- Exam mode
- AI features such as an AI assistant and an MCP server
- Plugins with signed manifests that can add panels, floating windows and shortcuts.
- 13 built-in themes, installable themes and resource packs.
- Performance settings, including a frame rate limit and a low performance mode.
- Automatic updates.

## Building

Everything goes through `./dev.sh`. Run it without arguments to see the commands, for example:

- `./dev.sh dev` starts the dev servers and the desktop app.
- `./dev.sh macos` and `./dev.sh windows` build the apps into `dist/`.
- `./dev.sh ios` builds the iPad app on a simulator.
