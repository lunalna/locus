// macOS opens new windows through the menu bar, other systems through these shortcuts.
addEventListener(
    "keydown",
    (event) => {
        if (event.ctrlKey && !event.altKey && !event.metaKey && event.code === "KeyN") {
            event.preventDefault();
            event.stopPropagation();
            window.locusOpenApp(event.shiftKey ? "suite" : "classic");
        }
    },
    true,
);
