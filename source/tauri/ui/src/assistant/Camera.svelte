<script>
    import { onMount } from 'svelte';
    import { Button } from '$lib/components/ui/button';
    import { assistant, attach } from '../lib/assistant.svelte.js';

    let video = $state();
    let hint = $state('Or press any key to take a picture.');

    onMount(() => {
        let stream = null;
        let stopped = false;
        navigator.mediaDevices
            .getUserMedia({ video: true })
            .then((result) => {
                stream = result;
                // The view may have closed while the camera was starting.
                if (stopped) stop();
                else video.srcObject = stream;
            })
            .catch((error) => (hint = `The camera is not available: ${error.message}`));
        const stop = () => stream?.getTracks().forEach((track) => track.stop());
        // Keys typed in the shadow root only, so GeoGebra's shortcuts are left alone.
        const root = video.getRootNode();
        root.addEventListener('keydown', onkeydown);
        return () => {
            stopped = true;
            stop();
            root.removeEventListener('keydown', onkeydown);
        };
    });

    function capture() {
        if (!video.videoWidth) return;
        attach(video, video.videoWidth, video.videoHeight);
        assistant.focusPrompt = true;
        assistant.view = 'conversation';
    }

    // Modifier keys alone are skipped, since Alt and others are app shortcuts.
    function onkeydown(event) {
        if (['Shift', 'Control', 'Alt', 'Meta'].includes(event.key)) return;
        event.preventDefault();
        capture();
    }
</script>

<div class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-4">
    <video bind:this={video} autoplay playsinline muted class="min-h-0 max-w-full rounded-lg"></video>
    <Button onclick={capture}>Capture</Button>
    <p class="text-muted-foreground">{hint}</p>
</div>
