# brickrail-2

A Rust/Bevy rewrite of brickrail: a layout editor and train-control simulation for LEGO train layouts, split into a server (simulation and control) and a client (editing and visualization).

<!-- driftless v3 -->
This file is **the glossary**: the project vocabulary that RFCs and ADRs
reference instead of re-defining. Definitions stay tight — 1–2 sentences, what
a term *is*, not how it works; an optional backlink points at the RFC/ADR where
the term was developed. Terms are grouped by area below.
<!-- /driftless -->

## Data tiers & modes

**Static layout**:
Tracks, connections, blocks, markers, train definitions, destinations and schedules — everything serialized in the layout file. Frozen while in control mode. ([ARCHITECTURE.md](ARCHITECTURE.md))

**Persistent state**:
State that survives across simulation runs but is not part of the layout, chiefly train block positions. Cached to disk separately from the layout; a cache, not a save file.

**Control state**:
Runtime simulation state owned by the server: locks, leg queues, wait times, marker progress. Initialized fresh on entering control mode.

**Layout data component**:
The serializable, editor-owned half of an element's data. Read-only during control mode.

**Simulation data component**:
The simulation-owned half of an element's data, created on entering control mode and removed on exit. Never serialized with the layout.

**Edit mode**:
Client-only mode in which the user manipulates the static layout. No server or simulation runs.

**Control mode**:
Mode in which the layout is handed to the server, which owns all control state; the client renders and sends commands.

## Layout elements

**Track**:
The atomic layout element; a positioned, oriented piece of rail identified by `TrackID`.

**Connection**:
A link between two tracks; the connection graph is what pathfinding is derived from.

**Logical graph**:
The facing-aware graph over directed tracks that pathfinding and leg construction operate on.

**Marker**:
A marked position on a track that a train's sensor detects when passing. Markers are the only feedback from the physical layout; at most one per track, identified by its `TrackID`. ([train-simulation.md](docs/train-simulation.md))

**Marker color**:
The color of a marker tile, used only for hardware-level detection and validation; carries no simulation meaning. A marker may be a wildcard.

**Block**:
A user-defined continuous section of tracks that can safely contain a whole train; the lockable unit of the simulation. Modeled as in Rocrail. ([train-simulation.md](docs/train-simulation.md))
_Avoid_: "block section" in the real-life dispatching sense.

**Section**:
A block's ordered list of directed tracks. Static — stored in the layout, not computed.

**Endpoint markers**:
The two markers a block owns at the ends of its section, marker A on the first track and marker B on the last. Spawned and removed with the block.

**Canonical enter marker**:
The endpoint marker whose detection means the train body has fully entered the block. Which one it is depends on travel direction and facing; every other marker role is defined relative to it.

**Passthrough speed**:
Per-direction block configuration giving the target speed for a train passing through without stopping. Baked into leg markers at construction.

**Train**:
A physical or virtual train entity, identified by a numeric `TrainID`. Layout data holds its name and hardware configuration; runtime state lives in the simulation tier.

**Facing**:
Whether a train's sensor sits on the leading or trailing side relative to its travel direction. Determines the canonical enter marker.

**Registry**:
A per-type map from domain ID to entity, needed for ID-based lookup such as layout loading. ECS relationships are preferred over registries at runtime.

## Movement

**Route leg**:
One block-to-block traversal and the atomic unit of train movement. Self-contained: it carries its block sections, travel section, markers and speeds. ([train-simulation.md](docs/train-simulation.md))

**Idle leg**:
A leg whose start and target are the same block, with an empty travel section and only the block's enter marker. How a stationary train is represented.

**Logical block**:
A block together with a travel direction and facing; the node type of the leg graph.

**Leg graph**:
The graph whose nodes are logical blocks and whose edges are legs. Derived purely from the layout; planning operates on it, never on the track graph directly.

**Sensor trajectory**:
The path the train's sensor follows from one canonical enter marker to the next. Legs collect their markers along it, as opposed to along the full block sections.

**Travel section**:
The tracks between a leg's start and target blocks. May have zero to many markers.

**Marker role**:
The function a marker serves within one leg: Exiting, Entering, Entered, or none. A property of the leg, not of the marker.

**Train-block position state**:
A train's relationship to a block: Outside, Entering, Entered, or Exiting, defined in terms of the train body, not the sensor.

**Baked-in speed**:
The `TrainSpeed` (Slow, Cruise, Fast) attached to each leg marker at construction, resolved from block passthrough speeds. Overridden by the driver when the train intends to stop.

**Leg queue**:
The legs a train has committed to, in traversal order. Movement operates on the queue alone and knows nothing of locks or other trains.

**Commit**:
Appending a leg to a train's queue. Only the planner commits, and only once it holds the leg's locks, so every queued leg is safe to traverse.

**Leg advancement**:
The transition from the current leg to the next, inferred by the simulation when the Entered marker is hit and a next leg exists. Completes the previous leg.

## Planning

**Planner**:
The layer that picks the next leg from a plan, acquires its footprint, and commits it to the leg queue. Owns the lock table. ([train-simulation.md](docs/train-simulation.md))

**Footprint**:
The tracks a leg needs exclusively: its start block section, travel section and target block section. Held in full until the leg completes.

**Lock table**:
The planner's per-track record of which train holds which track. Legs conflict only through overlapping footprints.

**Plan**:
A subgraph of the leg graph rooted at the last committed leg's target block in which every path leads to the destination. Only the committed queue is fixed; the plan beyond it may be revised.

**Destination**:
A target a train wants to reach: a block or set of acceptable blocks, optionally constrained by direction and facing. Drives planning.

**Strategy**:
A pluggable policy that assigns destinations to trains over time, such as a schedule strategy or a random strategy. Produces destinations only.

## Driver

**Train driver**:
The abstraction that receives leg data, executes it autonomously, and reports marker hits back to the simulation. Hardware-agnostic interface. ([train-simulation.md](docs/train-simulation.md))

**Virtual driver**:
A driver that simulates travel by advancing a continuous position against marker positions, producing the same events as hardware.

**BLE driver**:
A driver backed by a MicroPython train hub that reacts to sensor detections locally. Receives only committed legs.

**Marker hit**:
The event a driver reports when the train passed a marker; the sole driver-to-simulation signal.

## Commands & events

**Command**:
The canonical entry point for any state mutation, whether from the GUI, scripts or property tests. An ECS entity with a Pending → Completed / Failed lifecycle. ([architecture.md](docs/architecture.md))

**Layout command**:
A command that modifies the static layout, valid only while the simulation is stopped. Undoable and kept in undo history.

**Control command**:
A command that crosses into simulation territory, valid only while the simulation runs. Not undoable; despawned once its result is read.

**Command response**:
The simulation's reply to a control command, correlated by `CommandId`.

**State event**:
A `SimulationEvent` describing a resulting state change, not its cause, keyed by domain IDs. The only way simulation state is mutated, on server and client alike. ([train-simulation.md](docs/train-simulation.md))

**Transport**:
The plugin layer that moves commands, events and responses between client and simulation: SubApp extract today, network later. Core plugins never know which.

**Position interpolation**:
Client-side smoothing of train position between discrete state events. The server never sends continuous position.

## Client

**Headless layer**:
Commands, layout state, simulation and mode state — everything testable without rendering or UI. ([client-design.md](docs/client-design.md))

**Visual layer**:
Widgets, edit buffers, rendering, selection and input mapping. Reads ECS state freely; writes only through commands.

**Application state**:
Whether the app is editing or running a simulation. Lives in the headless layer; transitions are commands.

**Interaction state**:
Active tool, selection and in-progress edit buffers. Purely visual-layer; commands never reference it.

**Edit buffer**:
A widget's working state that accumulates an edit and flushes as a single command on commit.

## Testing

**Property test**:
A `proptest` case that generates random layouts or scenarios and asserts layout, route or simulation invariants. ([property-testing.md](docs/property-testing.md))

**Visual replay**:
Re-running a failed property-test seed in the client with controlled time to watch the scenario play out.
