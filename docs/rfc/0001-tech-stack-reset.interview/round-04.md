# Tech stack reset — round 4

Builds on [round 3](round-03.md): files, edit mode in the core, the crate
layout, and the daemon serving the UI are settled. Three leaves remain before
the frontier is empty.

### Q13 — UI stack specifics

Narrows [Q3](round-01.md) and [Q10](round-03.md). TypeScript, with the core in
wasm doing geometry and state. Two choices left on the UI side:

(a) **Layout canvas: Canvas2D or SVG.** Canvas2D: an imperative draw loop,
pan and zoom as one transform, hit-testing via core queries, scales to many
thousands of shapes. SVG: the browser does hover and hit-testing, crisp text,
simplest to get right, fine up to a couple of thousand elements. Since picking
is a core query either way, SVG's built-in hit-testing buys little here.

(b) **Panel framework.** React (what agents know best, largest ecosystem),
Svelte or Solid (fine-grained reactivity suits a live view model, smaller),
or none (vanilla TypeScript with a small store). The panels are an inspector,
a toolbar and a few dialogs.

Which pair?

➡️ Canvas2D for the layout, with core-side picking. React for the panels:
agent fluency outweighs the elegance gain of Svelte or Solid for a UI this
size, and React's ecosystem covers color pickers and dialogs without custom
work.

> yes

---

### Q14 — Protocol versioning

Narrows [Q12](round-03.md) and [Q8](round-02.md). Because the daemon serves the
UI and the Tauri app bundles both, the UI and the host that it talks to are
always the same build. The old inbox item "event schema and versioning for the
client-server boundary" then has a simple answer: there is no cross-version
boundary to version. The one durable format is the layout file, which carries
its version field (Q9), and the sibling persistent-state file.

One edge case exists: a browser tab left open across a daemon upgrade on the
Pi sees a mismatched build on reconnect. It is rare, visible (the reconnect
fails or the UI shows nonsense, the user refreshes), and in development it
does not arise, since the Vite dev server serves the UI with hot reload
rather than the daemon. A build identifier in the connect handshake with a
self-reload on mismatch is a one-line nicety for later, not a design
decision.

Agreed that only the files are versioned, and the wire is not?

➡️ Yes. Versioning a wire whose two ends always ship together is work with no
beneficiary.

> yes

---

### Q15 — Testing

The old property-testing doc wanted generators for layouts, blocks, markers
and scenarios, invariants at the layout, route and simulation layers,
controlled time, and visual replay of a failed seed. The sans-IO core makes
all of that native: proptest drives core components directly, time is
whatever the test passes as `now`, and a failed seed is a recorded input
sequence the UI can replay through the same mirror.

Proposal for where tests live:

- **Core:** the bulk. Unit and property tests on the pure components; no
  runtime, no wasm.
- **wasm wrapper:** a smoke test that the module loads, applies an event and
  returns a view, so boundary breakage is caught without testing logic twice.
- **Host:** integration tests over the WebSocket with a virtual-driver
  simulation; BLE behind a trait with a fake.
- **UI:** little. Rendering is checked by eye; the logic it would need is in
  the core.

Is that the split, and does the visual replay stay a goal of the reset?

➡️ Yes to the split. Visual replay stays a goal but not a first-target
deliverable: it needs only a "feed this recorded input sequence" entry point
on the browser host, which falls out once recording exists (inbox).

> yes
