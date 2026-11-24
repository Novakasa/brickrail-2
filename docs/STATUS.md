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

### Layout & lifecycle

| Topic | Status | Next / open threads | Links |
|---|---|---|---|
| ECS element lifecycle (spawn/despawn via messages, registries) | shipped | Move marker↔block associations from registries to ECS relationships | [ARCHITECTURE.md](../ARCHITECTURE.md) |
| Logical graph for facing-aware pathfinding | shipped | Rebuild runs in `Last`; needs explicit ordering or a dedicated schedule | |
| System scheduling | active | Replace brittle implicit ordering of the logical-graph rebuild | |

### Simulation

| Topic | Status | Next / open threads | Links |
|---|---|---|---|
| Route building (A*, leg construction) | shipped | Leg graph as the planning substrate; current impl walks one linear path | [train-simulation.md](design/train-simulation.md) |
| Train position state (placement, marker hits, leg completion) | shipped | | [train-simulation.md](design/train-simulation.md) |
| Driver interface + virtual driver | shipped | BLE driver not started | [train-simulation.md](design/train-simulation.md) |
| Locking / planner | design | Per-track lock table owned by the planner; footprint released when the train passes the Entered marker, whether or not a next leg is queued | [train-simulation.md](design/train-simulation.md) |
| Plans, destinations, strategies | design | Leg-graph subgraph plans; schedule and random strategies | [train-simulation.md](design/train-simulation.md) |
| SimulationEvent sync pipeline (fan-out, collector, SubApp extract) | shipped | | [architecture.md](architecture.md) |
| Networking transport for SimulationEvent | idea | Replace SubApp extract with a network transport; core plugins stay unchanged | [architecture.md](architecture.md) |
| High-level plugin composition (simulation, client, communication layer) | design | Three tiers per element type; one top-level state plugin and one logic plugin | [ARCHITECTURE.md](../ARCHITECTURE.md) |

### Commands & client

| Topic | Status | Next / open threads | Links |
|---|---|---|---|
| Editor→simulation command channel | design | Command entities in client, typed enum for simulation requests; start with `SendTrainToBlock` | [command-layer-design.md](command-layer-design.md) |
| Visualization client (gizmo-based track/block/train rendering) | shipped | Rendering approach open: gizmos vs. lyon vs. custom meshes | [client-design.md](client-design.md) |
| Train interpolation between marker hits | idea | Client-side only; server never sends continuous position | [client-design.md](client-design.md) |
| Client picking (hover/click) | idea | Needs a generic approach across element types | [client-design.md](client-design.md) |
| Inspector panel | idea | egui vs. bevy_ui; how selection drives the active edit buffer | [client-design.md](client-design.md) |

### Testing

| Topic | Status | Next / open threads | Links |
|---|---|---|---|
| Property-based testing | design | Generators for layouts, blocks, markers, train scenarios; controlled time for visual replay | [property-testing.md](property-testing.md) |

## Inbox / Unfiled

- Exact scope of persistent state: just train block positions, or more?
- Controlled time mode (fixed timestep / mock `Time`) for pause, slow-mo, stepping, and deterministic replay
- Evaluate the handwritten Rust Pybricks protocol implementation: rewrite, or sane enough to reuse? Written with little Rust async experience
- Event recording switch: dump state events to a file for replay and debugging (RFC-0001)
