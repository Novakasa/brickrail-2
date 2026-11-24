---
id: RFC-0001
title: Tech stack reset — a framework-free core with web and native hosts
status: active
---

# RFC-0001 — Tech stack reset

## Context

brickrail-2 was built Bevy-first: ECS on both the simulation side and the
client side, so that both could share one backbone, the same types, the same
plugins, the same queries. About five thousand lines exist in that shape:
layout lifecycle, leg construction, train position, a virtual driver, a
command layer, and a SubApp transport between client and simulation worlds.

The train simulation itself is a discrete-event control system: a totally
ordered log of state events keyed by domain IDs, a lock table, leg queues,
commands with correlation IDs. The code shows the mismatch between that and a
frame-scheduled ECS: an `Entering` state that waits one frame for registries to
fill, brittle implicit ordering of the logical-graph rebuild, fan-out systems
and ID-to-entity registries on both sides, command entities and a command
registry, a SubApp extract as transport. The scale is a handful of trains and
a few hundred tracks; nothing here needs ECS iteration performance.

Bevy's remaining value was the client renderer, and a Bevy web build runs to
tens of megabytes of wasm to draw a two-dimensional track diagram. Bluetooth
hardware control would have left Bevy anyway for an async runtime. Bevy also
ships breaking releases every few months, which taxes agent-maintained code.

This repository is the third brickrail. The first, on Godot, ran Bluetooth in
a separate Python process over WebSockets because Godot had no BLE; shipping
that process with its environment and keeping it running was a major
motivation for the first Rust rewrite, which embedded Bluetooth in the binary.

## Goals and biases

The developer's goals for the reset:

- **Simplicity** first.
- **Decoupling** so that the different client/server arrangements and
  native-versus-web clients are possible without reshaping the core.
- A **web client is first-class**; native apps are valued too.
- **No database and no Spacetime.** Layout, persistent state and event log
  are files or in-process state.

## Proposal

The shape the deliberation settled on; the decision-log below is authoritative.

**The simulation is a pure library component** (settled). It reads no clock,
does no I/O and knows no driver implementation. Its inputs are commands and
marker hits; its outputs are state events and driver instructions, a leg for a
train's driver to execute. Whoever embeds it delivers inputs and routes
outputs.

**Simulation state and simulation behaviour are split.** Simulation state is
a plain struct with a single mutation path, applying a state event; that
struct with its apply function is the state mirror. The simulation component
holds its own copy of that state and mutates it only by applying the events
it emits, so the event stream a client receives is complete by construction.
A client embeds only the mirror and runs no behaviour.

**The core is a library of such components**: the simulation, the virtual
driver, the state mirror that applies state events, and the editor geometry
(snapping, nearest track, section between two tracks, leg previews). The
virtual driver is advanced by its host with elapsed time and produces marker
hits that enter the simulation like a hub's. Property tests drive components
directly, with no runtime to boot; controlled time and replay fall out.

**Hosts around the core.** Each host owns I/O, clocks and routing:

- a native daemon (tokio, btleplug for BLE in-process, WebSocket for clients);
- a Tauri shell hosting the web UI with the daemon code in-process, which also
  gives native BLE since webviews expose no Web Bluetooth;
- the browser, with the core compiled to wasm, for a serverless virtual mode
  and, later, possibly Web Bluetooth hardware mode.

**Client.** A TypeScript web UI (settled): Canvas2D for the layout with
picking done by core geometry queries, a mainstream framework for inspector
panels. Types cross the wasm boundary as serde structures with generated
TypeScript declarations. The same UI runs in the browser and in the native
shell.

**Transport and sync.** The client holds a copy of simulation state and
applies the same state events the server applies. Reconnect is a snapshot
plus the event tail.

### Terms

**Core**: the library of pure components, the simulation, the virtual driver,
the state mirror that applies state events, and the editor geometry. No
component has a clock, performs I/O, or knows what sits on the other side of
its inputs and outputs.

**Host**: an executable that embeds some of the core components, passes them
time, performs all I/O, and routes: external happenings become core inputs,
core outputs go to their consumers. The daemon, the Tauri backend and the
browser are the hosts. The UI and the transport are not hosts; the UI consumes
state and issues commands, and the transport is one host's I/O talking to
another's.

## Alternatives considered

- **Keep Bevy.** Sunk cost is small; the ECS fights the event-log shape of the
  domain; the web bundle is heavy; churn is high.
- **Bevy only as renderer, inspector in the DOM.** Keeps the heaviest part of
  Bevy for the part the browser is best at; split focus and selection state
  across the wasm boundary; in a Tauri shell it becomes Bevy-in-wasm-in-webview,
  so native gains nothing. Worth revisiting only if a 3D view or rich animation
  becomes a goal.
- **Spacetime (formerly SpacetimeDB).** Model-level fit is unusually tight:
  transactional totally ordered reducers, tables as control state,
  subscriptions as sync, scheduled reducers as virtual-driver ticks, sandboxed
  wasm modules with the same constraints as browser wasm. BLE cannot run in a
  module, so a hub bridge process would remain. Rejected on the developer's
  simplicity goal: a young platform with fast churn, a relational reshaping of
  nested state, and a scale mismatch, against a plain alternative of a few
  hundred lines.
- **A database (SQLite) in the daemon** for event log and snapshots. Rejected
  for now under the no-database bias; files cover layout and persistent state.
- **egui/eframe** as a single Rust UI codebase. Lightweight and wasm-capable,
  but web becomes second-class.
- **GPUI** (Zed's framework, used by Discord's Rust client). Desktop only;
  Discord ships HTML for the web. Would mean a second UI codebase. Remains a
  possible desktop host swap later.
- **Vello or Rust-side Canvas2D** to keep drawing code in Rust without an
  engine. Drawing is the easy part; geometry lives in the core regardless.
- **A separable hub bridge** (BLE in its own process, as in the Godot-era
  brickrail). The one use case, range and connection count on big layouts,
  does not survive: BLE has no roaming, so a train moving out of one adapter's
  range cannot be handed to another, and connection count is solved by several
  adapters in one machine. In-process is the only option for the reset.
- **Lockstep mirroring.** With a fully deterministic simulation, clients could
  replay the server's inputs (commands and marker hits with timestamps)
  through their own copy of the behaviour instead of applying state events.
  Determinism would then be a sync correctness requirement, and any divergence
  (map iteration order, an unstable sort, a platform float difference) a
  silent desync. Mid-run joins still need a snapshot. The state-event mirror
  costs one struct and one apply function that the design has anyway, so
  determinism stays a testing and replay property.

## Open questions

None left from the deliberation. Implementation may reopen details, which
are folded into the body and decision-log while this RFC is active.

## Decisions

Confirmed so far:

1. Bevy is removed from the project. The reset removes everything Bevy
   touches, brings the docs to the target state first, and reimplements from
   the docs with agents that never see Bevy in context.
2. No database and no Spacetime.
3. Priorities: simplicity first, then decoupling for the client/server
   arrangements and for native versus web clients. The web client is
   first-class.
4. The train-simulation domain model stands as the design baseline and is
   not re-deliberated here.
5. The simulation is a pure library component with exactly two inputs,
   commands and marker hits, and two outputs, state events and driver
   instructions. It knows no driver implementation; the virtual driver is a
   separate component.
6. Required deployments: the combined native app, and a daemon near the layout
   with a remote client. Browser-only virtual mode is nice-to-have. Browser
   Web Bluetooth mode and mobile apps are out of scope for the reset but must
   not be designed against. The combined native app is the first target.
7. The client is a TypeScript web UI.
8. Bluetooth runs in-process in the host beside the simulation. A separable
   hub bridge is not supported.
9. Time enters the simulation only as the host's `now` carried on every input.
   The simulation may output a wake-up deadline, and the host calls back with
   a tick at that time. The simulation never reads a clock; waiting and
   scheduling stay simulation logic.
10. The native shell is Tauri with the daemon code running in-process, not as
    a sidecar. One UI codebase for browser and shell is a hard requirement,
    with only a thin platform layer (dialogs, menus) differing. The UI makes
    no desktop-only interaction assumptions, and the core, including the
    Pybricks codec, stays wasm-compilable, so Web Bluetooth and mobile remain
    possible.
11. The browser embeds the core compiled to wasm from the first target: the
    state mirror and editor geometry are Rust. TypeScript implements no
    apply-event and no geometry.
12. Client and simulation talk over WebSocket. The encoding is JSON behind a
    single function so it can change. On connect the server sends a full
    snapshot, then the event tail; no event log is persisted.
13. Files. The layout is a pretty-printed, versioned JSON file at a
    user-chosen path. Persistent state is a sibling file beside the layout.
    The host that runs the simulation owns both files, so in the daemon
    deployment the daemon's machine is the layout's home and the UI opens and
    saves through the host.
14. Edit mode lives in the core: layout commands, validation and undo are
    Rust; TypeScript holds interaction state only and never mutates layout
    data. Geometry queries (snapping, picking, sections) and position
    interpolation are core queries. Edit-mode and control-mode commands are
    the same kind of input, gated by mode. The simulation always assumes a
    static layout: the layout it receives on entering control mode does not
    change while it runs.
15. Workspace: `brickrail-core` (pure components, wasm-compilable),
    `brickrail-wasm` (cdylib wrapper, no logic), `brickrail-host` (tokio,
    BLE, WebSocket, files, UI serving), `brickrail-daemon` (binary),
    `brickrail-app` (Tauri over the host library), `web/` (TypeScript, Vite).
    The core never depends on wasm-bindgen; geometry stays inside the core.
16. The daemon serves the built web UI over plain HTTP on a configurable port
    with the WebSocket on the same port. LAN only, no authentication, no TLS,
    discovery by typed address.
17. UI stack: Canvas2D for the layout canvas with core-side picking; React for
    the panels.
18. Only the files are versioned: the layout file and its sibling persistent
    state file. The wire between UI and host is not, since both ends always
    ship as one build (the daemon serves the UI, the Tauri app bundles both).
19. Tests live mostly in the core as unit and property tests on the pure
    components; the wasm wrapper gets a smoke test; the host gets integration
    tests over the WebSocket with a virtual-driver simulation and BLE behind a
    trait with a fake; the UI gets little. Visual replay of a recorded input
    sequence stays a goal, not a first-target deliverable.
