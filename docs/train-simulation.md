# Train Simulation Domain Model

This document defines the domain model for markers, blocks, trains, and route legs — the core types that drive the train simulation.

## Markers

A **marker** is a marked position on the layout that a sensor on a train can detect as it passes over. Markers are the only feedback mechanism from the physical layout — they form the basis for all train state transitions.

- One marker per track (at most).
- `MarkerID = TrackID` — a marker is identified by the track it sits on.
- Markers serve different logical roles depending on context (block boundary, progress indicator, etc.), but the marker itself doesn't know its role — that's determined during leg construction.

### Hardware Implementation

In the current hardware setup, markers are colored tiles placed on the track, and trains carry a color sensor. Each marker has a **color** (`MarkerColor`: `Red | Yellow | Green | Blue | Cyan | White | Black`). Matching the detected color against the expected color provides a safety check, but a marker can also be configured as a wildcard (any color matches). The simulation logic doesn't assign meaning to specific colors — color is purely for detection and validation at the hardware level.

## Blocks

A **block** is a user-defined continuous section of tracks that can safely contain a train. Everything outside the block can be unlocked for other trains. This aligns with how Rocrail models blocks, not with real-life dispatching block sections.

- A block is defined by its **section**: an ordered `Vec<DirectedTrackID>`.
- `BlockID = { track_a, track_b }` (normalized endpoints of the section) — a convenient unique identifier derived from the section.
- Block sections are **static** — stored in the layout, not computed from the connection graph.
- The section between a block's two endpoint markers must be long enough to fully contain any train.

### Endpoint Markers

When a user creates a block, two **endpoint markers** are automatically spawned — one at each end. These markers are owned by the block (created with it, removed with it) and signal to the user that physical markers should be placed at these positions.

### Blocks at Runtime

Blocks are the **lockable unit** of the simulation: they define which sections need to be locked and unlocked in response to marker events. Route legs are structured around blocks (start block → travel → target block), and the planner uses block identity to determine:

- Which sections to lock ahead of the train
- When to release locks behind the train (only after the train has **entered the target block**, not merely when it has "left" the source block — because travel sections between blocks are not guaranteed to be long enough to contain the train)

### Per-Direction Configuration

Blocks have configuration that varies by **travel direction** (aligned or against the section direction). This is stored as two `DirectedBlockConfig` values on the block data.

Currently this includes **passthrough speed** — a target speed that applies when a train passes through the block without intending to stop. During leg construction, the passthrough speed for the relevant travel direction is baked into the leg's markers; if the train actually stops in the block, the driver overrides it (see Marker Speed). This allows modeling speed adjustments for track features — slowing down for curves, or speeding up to climb ramps (where one direction needs fast and the other slow).

### Train-Block Position States

A train's relationship to a block has 4 states:

1. **Outside** — train is elsewhere.
2. **Entering** — the train's leading end has crossed the boundary, trailing end hasn't fully entered.
3. **Entered** — entire train is within the block.
4. **Exiting** — the train's leading end has crossed the far boundary, trailing end hasn't fully left.

These states are defined in terms of the train body relative to the block, not the sensor. Note that the marker role names (Entering, Entered, Exited) are related but distinct — they name the *event* that a marker triggers, not the ongoing train-block state. The Entered marker fires at the moment the train transitions from Entering to Entered state. The Exited marker fires in the *next leg's* context, signaling that the train has left the previous block.

### Canonical Enter Marker

Each block has two endpoint markers: **marker A** (on the first track of the section) and **marker B** (on the last track). The **canonical enter marker** — the marker whose detection signals the "entered" state transition — depends on both **travel direction** and **facing**:

| Travel direction       | Facing   | Enter marker |
|------------------------|----------|--------------|
| Aligned with section   | Forward  | B (far end)  |
| Aligned with section   | Backward | A (near end) |
| Against section        | Forward  | A (near end) |
| Against section        | Backward | B (far end)  |

The logic: the "entered" marker is the one the sensor crosses when the train body has fully entered the block. A forward-facing sensor is on the leading side, so it crosses the far boundary first. A backward-facing sensor is on the trailing side, so it crosses the near boundary only after the body has already moved deep into the block.

All other marker roles in a route leg are defined by their position **relative to** this canonical enter marker — not by a fixed index shift. Markers before the enter marker (in travel order) may signal "entering" or speed changes; markers after it may signal "leaving". If additional markers exist inside the block section (between A and B), they naturally fall into the correct role based on which side of the enter marker they sit on.

## Trains

A **train** is a physical or virtual train entity.

- `TrainID = u32` — a numeric identifier. The train's human-readable name is stored in data.
- Layout data includes the train name and hardware configuration (hub kind, channel, speed calibration — to be expanded later).
- Runtime state (position, speed, leg queue) belongs to the simulation tier, not the layout.

### Train Facing

A train has a **facing** relative to its travel direction — it determines whether the sensor is on the leading or trailing side of the train. Facing affects which marker serves as the canonical enter marker for a given block (see Canonical Enter Marker above).

## Route Legs

A **route leg** is one block-to-block traversal. Legs are the unit of train movement: a train only ever moves across legs that have been committed to its queue (see Train Control Hierarchy below). Deciding *which* legs a train should take is a planning concern, handled separately — it produces legs but plays no part in moving the train.

### Sensor Trajectory and Train Body

Pathfinding operates on the **sensor trajectory** — the path the train's sensor follows from one block's canonical enter marker to the next. This is the primary data for each route leg. Markers are collected along this trajectory, since the sensor is what detects them.

However, the train has non-zero length — its body extends behind the sensor. A route leg therefore also stores the **full block sections** (start and target) from block data, even though the sensor may only traverse part of them. These sections are needed for **locking**: the entire block must be reserved to ensure the train body fits, not just the tracks the sensor crosses.

### Leg Contents

Each route leg stores:

1. **Start block section** — the full section of the starting block (for locking).
2. **Travel section** — the tracks between the start and target blocks (from the sensor trajectory).
3. **Target block section** — the full section of the target block (for locking).
4. **Markers** — collected along the sensor trajectory, not from the full block sections.

### Marker Collection and Roles

Markers are collected along the sensor trajectory (enter marker to enter marker) and assigned roles by position. Each leg has at most one marker of each role (or none if there aren't enough markers). The three marker roles align with the train-block states:

- **Exiting**: the first marker in the leg — the canonical enter marker of the start block. The train starts here and is exiting the start block.
- **Entering**: the marker immediately before the Entered marker in travel order — signals the train's leading end is crossing into the target block area.
- **Entered**: the last marker in the leg — the canonical enter marker of the target block. Signals the train has fully entered the target block. Anchors leg completion (and therefore lock release).

A typical leg's marker sequence:

```
[Exiting] → ...no role... → [Entering] → [Entered]
```

The same physical marker is **Entered** at the end of one leg and **Exiting** at the start of the next — its role depends on which leg it belongs to.

All remaining markers (those between Exiting and Entering) have **no role**, corresponding to the **Outside** train-block state — the train is between blocks. These markers are used only for visual progress interpolation.

Marker roles are a **route leg concern**, not a property of the marker itself. The same physical marker can have different roles in different route legs.

### Marker Speed

Each marker in a route leg has a **baked-in speed** (`TrainSpeed`: `Slow | Cruise | Fast`) resolved at leg construction time. This is the target speed the train should travel at after passing this marker. The speed is determined by the marker's role:

- **Exiting** — passthrough speed of the **start block** (for the relevant travel direction).
- **Entering** / **Entered** — passthrough speed of the **target block** (for the relevant travel direction).
- **No role** — `Cruise` (default speed for travel sections between blocks).

Speeds are resolved once when the leg is built, not looked up dynamically at runtime. This keeps the leg self-contained — the driver (virtual or BLE) receives exactly the speeds it needs without consulting block metadata.

When a train intends to stop at the current leg (i.e. no next leg is queued), the driver overrides the baked-in speeds: it slows down after the **Entering** marker and stops after the **Entered** marker, regardless of the speeds assigned to those markers. The baked-in speeds only apply during pass-through.

### Travel Section Markers

- Travel sections can have **0 to many** markers.
- If a travel section has no markers near a block boundary, the adjacent block's boundary marker serves as the Exited or Entering marker instead. This is a **conservative fallback** — the train will be in transitional states for longer, but correctness is preserved.
- Extra markers in travel sections (beyond the minimum needed) help with showing visual progress but don't change block state logic.

### Locking

Locking is a **planner concern**. Train movement never sees it: a leg in a train's queue is driven on the assumption that nothing else is in the way, and it is the planner's job to make that assumption true before committing the leg.

Each leg has a **footprint** — the tracks it needs exclusively:
- The **start block section**
- The **travel section**
- The **target block section**

The planner holds a per-track lock table. Before committing a leg to a train's queue, it acquires the leg's whole footprint; a leg whose footprint overlaps another train's holdings cannot be committed. Legs that share tracks — at every switch and crossing — conflict through their footprints, never through any knowledge the train has of other trains.

Locks are **not** released progressively as the train's rear clears each section. A leg's footprint is held until the leg is complete — the train has entered the target block and advanced onto the next leg. This is because travel sections between blocks are not guaranteed to be long enough to contain the train — only blocks provide that guarantee. On completion, the planner releases whatever the leg held that no remaining leg of the same train still needs (the target block section is also the next leg's start section, so it stays held). The planner learns of completion from the leg advancement that movement already performs; movement does not call into the planner.

## Leg Construction

A leg connects two **adjacent** logical blocks: its sensor trajectory is a path of logical tracks from one block's canonical enter marker to the next, with no other enter marker in between. Leg construction is local — it needs only that path slice and the block data at both ends:

1. Resolve the full block sections of the start and target blocks from block data (for locking).
2. Extract the travel tracks from the path slice.
3. Collect markers along the sensor trajectory (the path slice, not the full block sections).
4. Assign marker roles by position within the leg.

Each leg is self-contained — its block sections, markers, and speeds depend only on the blocks it connects, not on any larger path or plan it is part of.

### Leg Graph

Taken together, all possible legs form the **leg graph**: nodes are logical blocks (block, direction, facing), edges are legs. Two nodes may be connected by several edges when parallel track paths exist between the blocks. The leg graph is derived purely from the layout, so it can be computed up front (or lazily, as legs are needed) and only changes when the layout does.

Planning operates on the leg graph, never on the track graph directly. Searching for a single track-level path to a target and splitting it at enter markers is equivalent to walking one path through the leg graph — a special case, and how the current implementation builds a linear plan.

Whether legs are built once per layout and shared between trains, or built on demand per train, is a decision internal to planning and leg construction. Train movement — the leg queue, leg advancement, the driver — only ever sees legs in a train's queue and must not depend on where they came from.

## State Events

Simulation state is **event-sourced**. The server holds the authoritative state and can only mutate it by emitting state events. These same events are forwarded to clients, who apply the same mutations to their own state copies.

State events describe **resulting state changes**, not causes. The server's simulation logic decides what happens; the emitted events describe the outcome. This keeps mutation logic trivial — the client applies state deltas mechanically without understanding simulation logic.

State events use **domain IDs** (`TrainID`, `BlockID`, `TrackID`), not ECS entities. Both server and client resolve IDs to entities via their own registries. This allows state events to be serialized and sent over the network.

Examples of state events:
- `AppendLegs` — append pre-built route legs to a train's queue
- `LockAcquired` / `LockReleased` — block/track lock changes (future)
- `TrainStateChanged` — train movement state transitions (future)
- `MarkerPassed` — train sensor passed a marker (future)

## Train Control Hierarchy

Train behavior is driven by a layered abstraction, from low-level to high-level:

### Leg Queue

The **route leg** is the atomic unit of train movement — one block-to-block traversal. A train is always assigned to a route leg. A stationary train has a single-block **idle leg**: start and target are the same block, the travel section is empty, and the only marker is the block's enter marker. The idle leg is how a stationary train is represented — its position is just a logical block, with no memory of where it came from — and advancing onto it is what completes the previous leg on arrival (letting the planner release its locks), exactly as advancing onto any other leg does.

Each train has a **leg queue**: the legs it has committed to, in traversal order. The train executes legs in order, advancing to the next leg as it enters each target block. Appending a leg to the queue *is* the commit: the planner only appends a leg once it holds the leg's locks, so everything in the queue is safe to traverse. Movement — leg advancement and the driver handoff — operates on the queue alone. It has no notion of locks or of other trains; a leg in the queue is driven, full stop. Lock data may live alongside leg data in the ECS for convenience, but no movement system reads it.

### Plans

A **plan** is the planning layer's view of where a train is going: a subgraph of the leg graph rooted at the last committed leg's target block, in which every path leads to the destination. The planner's job is to pick an outgoing edge from the root, acquire its footprint, and commit it to the leg queue. The planner owns the lock table (see Locking); movement never touches it.

A plan may be a single linear path or a graph with alternatives — which one depends on how the planner searches the leg graph (shortest path, all paths within a cost bound, etc.). With alternatives, the train makes dynamic decisions about which leg to commit next — if one branch is blocked by another train's locks, the planner can commit a different edge toward the same destination without discarding the plan. Only the committed queue is fixed; the plan beyond it can be revised at any time.

### Destinations

A **destination** is a target the train wants to reach. It can be:
- A single block
- A set of acceptable blocks (any one satisfies the destination)
- Optionally constrained by target direction and/or facing

A destination drives planning: the planner searches the leg graph from the train's current logical block to the destination and produces a plan. The plan serves the destination, not the other way around — it can be revised or replaced as long as it still reaches the destination.

### Strategies

The highest level of control assigns **destinations** to trains over time. A **strategy** is a pluggable policy that produces destinations:
- A **schedule strategy** assigns a fixed sequence of destinations (e.g. stop A → stop B → stop C → repeat).
- A **random strategy** assigns arbitrary destinations periodically.
- Other strategies can be added (e.g. demand-driven, priority-based).

Strategies only produce destinations — they don't interact with plans or legs directly.

## Train Driver

The **train driver** is the abstraction that bridges the simulation logic and the physical (or simulated) train. It receives leg data and autonomously executes it, reporting events back to the simulation state layer.

### Why an Abstraction?

The simulation state layer (`TrainPosition`, `TrainLegState`, `TrainMarkerHit`, `AdvanceLeg`) is hardware-agnostic — it reacts to events without knowing what produced them. The driver is what produces those events. A consistent interface means the simulation logic works identically whether trains are physical BLE devices or virtual simulations.

### Interface

**Simulation logic → Driver:**
- Append a leg to the driver's queue (with marker data and facing)

**Driver → Simulation state:**
- `TrainMarkerHit` — the train passed a marker

The driver does not report leg advancement — the simulation logic infers leg transitions from marker events (specifically, when the Entered marker is hit and a next leg exists). The BLE train may advance legs locally for latency reasons, but the server derives its own state independently from marker hits. A debug assertion can verify they stay in sync.

### Driver Behavior

The driver maintains a queue of legs. Each leg contains a sequence of markers with metadata (role, speed, color/position) and a facing direction. Its behavior is simple:

1. Execute the current leg — advance through its markers in order.
2. Marker metadata determines speed behavior — e.g. slow down at Entering, stop at Entered.
3. When the current leg is complete (all markers passed):
   - **If another leg is queued** → advance to it immediately (pass through).
   - **If no next leg** → stop.
4. Report each marker hit back to the simulation.

The driver's leg data is a thin subset of a `RouteLeg` — just markers and facing. It doesn't include block sections, block IDs, or locking information. Those stay in the simulation layer.

This means the planner controls when the train can proceed by controlling when it commits legs. Dispatch to the driver follows directly from a leg entering the train's queue. Neither the queue nor the driver knows about locks — the driver just knows whether it has more legs to execute.

### BLE Hardware Driver

BLE trains run MicroPython and are semi-autonomous. Due to BLE latency (especially with many trains), the train must react to sensor detections locally without round-tripping to the control PC.

The control PC downloads legs to the BLE train's queue. Each leg encodes:
- Marker sequence (expected colors for sensor validation)
- Marker roles (which markers trigger speed changes)
- Facing/direction

The BLE train's onboard logic handles:
- Sensor reading and color matching
- Speed control (accelerate, cruise, slow down, stop) based on marker roles
- Autonomous leg advancement when the next leg is already queued
- Stopping when no next leg is available

The train reports events back to the control PC asynchronously:
- Marker passed (with index)
- Unexpected marker (color mismatch — safety event)

Only committed legs — legs in the train's queue — are sent to the train. This replaces the previous design where all legs were sent and each had a dynamic `intent_stop` flag that could be toggled remotely. The new approach is simpler: if a leg is on the train, it's safe to traverse.

### Virtual Driver

The virtual driver simulates the same behavior without hardware. It uses the leg's marker positions and a simulated speed to advance through markers. Marker positions reflect actual layout distances so that simulated travel times are consistent across legs of different lengths.

Each simulation tick:
1. Advance the train's continuous position by `speed × delta_time`.
2. If the position crosses the next marker's position, emit `TrainMarkerHit`.
3. If the current leg is complete and a next leg exists, advance to it.
4. If the current leg is complete and no next leg exists, stop.

The virtual driver receives the same leg data as the BLE driver and produces the same events. The simulation logic doesn't distinguish between the two.
