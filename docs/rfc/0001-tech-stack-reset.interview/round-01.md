# Tech stack reset — round 1

Under interrogation: the proposal in [RFC-0001](../0001-tech-stack-reset.md),
starting at the root — the core's contract, the deployments that must exist,
the client technology, and where hardware I/O lives.

### Q1 — The simulation's inputs and outputs

This question is about the simulation only, the component that owns train
positions, leg queues, the lock table and the planner. Not the virtual
driver, not the client, not any host.

The simulation is a pure library component: it reads no clock, does no I/O,
and knows no driver implementation. It would have exactly two kinds of input
and two kinds of output:

- **In:** commands (send train to block, place train, enter control mode) and
  marker hits (a train's sensor passed a marker, whoever reported it).
- **Out:** state events (the mutations of simulation state, as in the design)
  and driver instructions (a leg for a train's driver to execute).

Whoever embeds the simulation delivers the inputs and routes the outputs. The
virtual driver is a separate component that the embedding program runs and
advances with time; its marker hits enter the simulation exactly like a hub's.

Is that the simulation's complete boundary, two inputs and two outputs, or is
something missing on either side?

➡️ Yes, that is the complete boundary. (Whether the simulation needs time for
waits or schedules, and how it would get it, is a round 2 question.)

> yeah
---

### Q2 — Which deployments must exist?

Decoupling has a price per arrangement, so the required set should be named.
Candidates:

1. **Combined native app**: UI and simulation on one laptop next to the
   layout, BLE from that machine.
2. **Daemon plus remote client**: the simulation and BLE run on a machine near
   the layout, such as a Raspberry Pi; the UI runs elsewhere, in a browser or a
   native shell.
3. **Browser-only virtual mode**: no daemon, the simulation runs in the
   browser with virtual trains.
4. **Browser hardware mode**: the browser talks to hubs directly over Web
   Bluetooth, no daemon. Chrome and Edge only.
5. **Mobile** native apps.

For each: required, nice-to-have, or out of scope? And if several are
required, which one is the first target the reset builds toward?

➡️ Required: 1 and 2. Nice-to-have: 3 (it costs almost nothing once the core
is wasm). Out of scope for the reset: 4 and 5, both possible later without
reshaping anything. First target: 1, because it exercises the full stack
including BLE on one machine.

> yes, but 4 and 5 are something I would consider later, so we should at least not go against it

---

### Q3 — Client technology: one web UI, or a Rust UI?

Given a first-class web client and valued native apps, three shapes:

- **TypeScript web UI, plus a Tauri shell for native.** One UI codebase.
  Native gets a system webview, native menus and dialogs, and BLE through the
  Rust side. Web gets the same UI unchanged.
- **egui/eframe.** One Rust codebase, desktop-first; a wasm build exists but
  the web version is a canvas-rendered app with weaker text, accessibility and
  mobile behavior.
- **GPUI** for desktop plus a separate web UI. Two UI codebases.

The earlier worry was editor quality in TypeScript. The proposal answers it
by keeping snapping, picking and section geometry in the core and leaving
TypeScript with pointer events, drawing and panels.

Which shape? And is "one UI codebase for web and native" a hard requirement
of the reset, or would a native-only Rust UI be acceptable if the web client
came later?

➡️ TypeScript web UI plus Tauri shell, and one UI codebase as a hard
requirement. It is the only shape where the web client is first-class and
native costs no second UI.

> typescript web ui

---

### Q4 — BLE in the host process: default, or the only option?

The hub side is Pybricks over BLE. The Pybricks message encoding and decoding
is pure and lives in the core; what varies is who holds the Bluetooth
connection and feeds hub reports into the simulation.

The project's history settles the default. The first brickrail, on Godot, ran
BLE in a separate Python process over WebSockets because Godot had no BLE.
Shipping that process with its environment to users, and keeping it running
and handling its errors, was a major motivation for the first Rust rewrite:
embed Bluetooth in the binary. So the default for the reset is in-process:
the daemon, and the Tauri backend, hold the hub connections themselves.

What remains open is whether a **separable hub bridge**, a small process that
only speaks BLE and talks to the simulation over the network, is something
the reset *supports*, with its own binary, orchestration and error handling,
or something it merely *keeps possible*. The cheap version of "possible": the
BLE adapter inside the host already talks to the rest of the host through
hub-level messages (connect, send leg, hub reported marker), so moving it
behind a socket later is a new host, not a redesign of the simulation.

Which is it for the reset: in-process only, with the bridge kept possible by
that message boundary; or in-process by default, plus the bridge as a
supported second deployment from the start?

➡️ In-process only, bridge kept possible. Every decoupling the reset *supports*
has to be built, documented and kept working; this one earns its place only
when a concrete arrangement needs the simulation away from the Pi.

> only option for now.
