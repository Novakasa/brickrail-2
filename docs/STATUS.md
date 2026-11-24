<!-- driftless v5 -->
# Project Status

Canonical, current source of truth for the state of each topic. **Consult it
when planning; capture new todos and ideas in the Inbox, not in agent memory.**
It's meant to churn; keeping it current is its whole job.

**Topics:** one row per topic in play. `Now` is one present-tense sentence,
rewritten on every touch; `Updated` is stamped today on every touch.
Statuses: `active` · `blocked` · `deferred` · `idea` — a topic that ships or
resolves leaves the table.
**Inbox:** append-only — a row is appended, promoted (to a topic row or a
draft RFC), or removed, never edited. Links point at the design
(`docs/design/`) and the frozen RFCs and ADRs; how-to guides stay in `docs/`.
Full rules: the `driftless:conventions` skill.
<!-- /driftless -->

## Topics

### Now

| Topic | Status | Updated | Now | Links |
|---|---|---|---|---|
| Tech stack reset | active | 2026-11-24 | The core crate is seeded with the carried modules; the simulation, hosts and UI are rebuilt from the design next. | [RFC-0001](rfc/0001-tech-stack-reset.md), [architecture](design/architecture.md) |

### Core

| Topic | Status | Updated | Now | Links |
|---|---|---|---|---|
| Layout model and layout commands | deferred | 2026-11-24 | Layout commands, validation and undo go on top of the seeded primitives and graphs. | [layout](design/layout.md), [architecture](design/architecture.md) |
| Editor geometry queries | deferred | 2026-11-24 | Snapping, picking, sections and position interpolation as core queries. | [architecture](design/architecture.md) |
| Simulation (leg queue, position, state events) | deferred | 2026-11-24 | Reimplement as a pure component with the two-input, two-output boundary; leg construction and position are seeded. | [train simulation](design/train-simulation.md) |
| Locking and planner | deferred | 2026-11-24 | Per-track lock table owned by the planner; footprints released on leg completion. | [train simulation](design/train-simulation.md) |
| Plans, destinations, strategies | deferred | 2026-11-24 | Leg-graph subgraph plans with schedule and random strategies. | [train simulation](design/train-simulation.md) |
| Virtual driver | deferred | 2026-11-24 | A host-ticked component producing marker hits from layout distances. | [train simulation](design/train-simulation.md) |
| Layout and persistent-state files | deferred | 2026-11-24 | Versioned JSON layout file with a sibling state file, owned by the simulation's host. | [architecture](design/architecture.md) |
| Property-based testing | deferred | 2026-11-24 | Generators and invariants run directly against core components. | [property-testing](property-testing.md) |

### Hosts and UI

| Topic | Status | Updated | Now | Links |
|---|---|---|---|---|
| Host library and daemon | deferred | 2026-11-24 | Routing, timers, WebSocket, file I/O and UI serving over the core. | [architecture](design/architecture.md) |
| BLE driver | deferred | 2026-11-24 | Pybricks codec in the core, btleplug transport in the host library. | [train simulation](design/train-simulation.md) |
| Desktop app | deferred | 2026-11-24 | Tauri shell with the host library in-process; the first deployment target. | [architecture](design/architecture.md) |
| Web UI | deferred | 2026-11-24 | TypeScript UI over the wasm core: Canvas2D layout, React panels. | [architecture](design/architecture.md) |
| wasm wrapper | deferred | 2026-11-24 | A cdylib exposing mirror, geometry and layout commands with generated TypeScript types. | [architecture](design/architecture.md) |
| Dev shell | deferred | 2026-11-24 | Rust with the wasm32 target and Node are in; wasm-bindgen-cli and Tauri build deps follow their crates. | |

## Inbox

| Captured | Item | Topic |
|---|---|---|
| 2026-10-10 | Exact scope of persistent state: just train block positions, or more? | Layout and persistent-state files |
| 2026-10-10 | Controlled time mode for pause, slow-mo, stepping, and deterministic replay | Host library and daemon |
| 2026-10-10 | Evaluate the handwritten Rust Pybricks protocol implementation from the previous rewrite: reuse or redo? | BLE driver |
| 2026-11-24 | Event recording switch: dump state events to a file for replay and debugging | Host library and daemon |
| 2026-11-24 | Ordered maps or a fixed hasher in the core so replays are deterministic | Simulation (leg queue, position, state events) |
| 2026-11-24 | Native app as a remote client of a daemon | Desktop app |
| 2026-11-24 | Daemon discovery by mDNS, or a printed URL and QR code at startup | Host library and daemon |
