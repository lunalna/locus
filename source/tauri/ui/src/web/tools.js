// The assistant's tools, run directly on the app in this page.

const object = (properties, required = Object.keys(properties)) => ({ type: 'object', properties, required });

const DEFINITIONS = [
    {
        name: 'locus_command',
        description:
            'Runs a command in the Algebra input, as if the student typed it there, so created objects appear in the Algebra and Graphics views, e.g. "f(x)=x^2" or "A=(1,2)". Returns the labels of created objects.',
        input_schema: object({ command: { type: 'string' } }),
    },
    {
        name: 'locus_view',
        description: "Shows or hides one of the app's views: algebra (the Algebra view with its input bar), graphics or cas.",
        input_schema: object({ view: { type: 'string', enum: ['algebra', 'graphics', 'cas'] }, visible: { type: 'boolean' } }),
    },
    {
        name: 'cas_evaluate',
        description:
            "Computes with the CAS without changing the student's construction, e.g. Solve(x^2=4), Intersect(x^2, 2x), Factor(x^2-1), Derivative(sin(x)) or Integral(x^2, 0, 1). Returns the result.",
        input_schema: object({ input: { type: 'string' } }),
    },
    {
        name: 'cas_add_row',
        description:
            "Adds a row to the student's CAS view and returns its output, e.g. f(x):=x^2 or Solve(f(x)=4). Objects defined there are not drawn in the Graphics view.",
        input_schema: object({ input: { type: 'string' } }),
    },
    {
        name: 'locus_screenshot',
        description: "Returns a picture of the student's Graphics view and its visible x and y range.",
        input_schema: { type: 'object', properties: {} },
    },
    {
        name: 'locus_viewport',
        description: 'Sets the visible area of the Graphics view.',
        input_schema: object({ xmin: { type: 'number' }, xmax: { type: 'number' }, ymin: { type: 'number' }, ymax: { type: 'number' } }),
    },
    {
        name: 'locus_objects',
        description: "Lists all objects in the student's construction with their type and value.",
        input_schema: { type: 'object', properties: {} },
    },
];

// Only Classic has a CAS view and Classic perspectives, and Scientific has no Graphics view.
export function definitions() {
    const app = window.appID || 'classic';
    const hidden = [];
    if (app !== 'classic') hidden.push('locus_view', 'cas_add_row');
    if (app === 'scientific') hidden.push('locus_screenshot', 'locus_viewport');
    return DEFINITIONS.filter((tool) => !hidden.includes(tool.name));
}

export function label(name, input) {
    const views = { cas: 'CAS', algebra: 'Algebra' };
    return {
        locus_command: () => `Ran ${input.command ?? ''}`,
        locus_objects: () => 'Read the construction',
        locus_screenshot: () => 'Looked at the Graphics view',
        locus_viewport: () => `Moved the view to x ${input.xmin} to ${input.xmax}, y ${input.ymin} to ${input.ymax}`,
        locus_view: () => `${input.visible === true ? 'Showed' : 'Hid'} the ${views[input.view] ?? 'Graphics'} view`,
        cas_evaluate: () => `Computed ${input.input ?? ''}`,
        cas_add_row: () => `Added CAS row ${input.input ?? ''}`,
    }[name]?.() ?? name;
}

// Checks the teacher's CAS setting and waits until the CAS engine has loaded.
async function casReady() {
    if (!window.ggbAssistantCasEnabled()) throw 'The CAS is disabled in this exam.';
    for (let i = 0; i < 60 && ggbApplet.evalCommandCAS('1+1') === '?'; i++) {
        await new Promise((resolve) => setTimeout(resolve, 500));
    }
}

const calls = {
    async locus_command(args) {
        ggbApplet.setErrorDialogsActive(false);
        try {
            return (
                (await ggbApplet.asyncEvalCommandGetLabels(args.command)) ||
                'No new objects were created. The command may have failed or only changed existing objects.'
            );
        } finally {
            ggbApplet.setErrorDialogsActive(true);
        }
    },
    locus_objects: () =>
        ggbApplet
            .getAllObjectNames()
            .map((n) => `${n} (${ggbApplet.getObjectType(n)}): ${ggbApplet.getValueString(n)}`)
            .join('\n') || 'The construction is empty.',
    locus_view(args) {
        const code = { algebra: 'A', graphics: 'G', cas: 'C' }[args.view];
        if (!code) throw 'Unknown view.';
        if (code === 'C' && !window.ggbAssistantCasEnabled()) throw 'The CAS is disabled in this exam.';
        ggbApplet.setPerspective((args.visible ? '+' : '-') + code);
        return 'Done.';
    },
    locus_viewport(args) {
        if (!(args.xmin < args.xmax && args.ymin < args.ymax)) throw 'The minimum must be below the maximum.';
        ggbApplet.setCoordSystem(args.xmin, args.xmax, args.ymin, args.ymax);
        return 'Done.';
    },
    async cas_evaluate(args) {
        await casReady();
        return ggbApplet.evalCommandCAS(args.input);
    },
    async cas_add_row(args) {
        await casReady();
        return window.ggbAssistantCasRow(args.input);
    },
    locus_screenshot() {
        const view = JSON.parse(ggbApplet.getViewProperties(1));
        const xMax = view.xMin + view.width * view.invXscale;
        const yMax = view.yMin + view.height * view.invYscale;
        return [
            { type: 'image', source: { type: 'base64', media_type: 'image/png', data: ggbApplet.getPNGBase64(1, false, 72, false, false) } },
            { type: 'text', text: `Visible area: x from ${view.xMin} to ${xMax}, y from ${view.yMin} to ${yMax}.` },
        ];
    },
};

// Returns tool result content: text, or image and text blocks for the screenshot.
export async function call(name, input) {
    if (!calls[name]) throw `Unknown tool: ${name}`;
    const result = await calls[name](input);
    return Array.isArray(result) ? result : String(result);
}
