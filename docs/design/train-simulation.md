# Train Simulation

The train simulation moves trains across a layout safely: it knows where every
train is, decides which moves are safe, and drives trains, physical or virtual,
through those moves. This file defines the model it works with, from layout
through movement and planning to the driver. The grid, tracks, connections and
the logical graph are defined in the [layout](layout.md) design.

## Markers

A **marker** is a marked position on a track that a sensor on a train detects
as it passes over. Markers are the only feedback from the physical layout; every
train state transition is grounded in a marker detection.

- A track carries at most one marker, and the marker is identified by its track.
- A marker has no role of its own. Which role it plays is a property of each
  route leg that includes it (see Marker Roles).

A marker has a **marker color**, used only for hardware-level detection and
validation: the detected color is matched against the expected one as a safety
check. A marker may be a wildcard that matches any color. Colors carry no
simulation meaning.

## Blocks

A **block** is a user-defined continuous section of tracks that can safely
contain a whole train, modeled as in Rocrail. Everything outside the block can
be released to other trains.
_Avoid_: "block section" in the real-life dispatching sense.

- A block is defined by its **section**: an ordered list of directed tracks.
  Sections are static, stored in the layout.
- The section between a block's two endpoint markers is long enough to fully
  contain any train.

A **logical block** is a block together with a travel direction and facing.

### Endpoint Markers

A block owns two **endpoint markers**, one on the first track of its section
(marker A) and one on the last (marker B). They are spawned and removed with the
block and show the user where physical markers go.

### Blocks at Runtime

Blocks are the **lockable unit** of the simulation. Route legs are structured
around blocks (start block → travel → target block), and the planner uses block
identity to decide which sections to hold ahead of a train and when to release
the sections behind it: only once the train has fully entered the target block,
because only blocks are guaranteed to contain a whole train.

### Per-Direction Configuration

A block's configuration varies by travel direction, aligned with or against its
section. It carries the **passthrough speed**: the target speed for a train
passing through the block without intending to stop. Leg construction bakes the
passthrough speed for the relevant direction into the leg's markers; a train
that stops in the block has the driver override it (see Marker Speed). This
models speed adjustments for track features, such as slowing for curves or
climbing a ramp fast in one direction and descending slow in the other.

### Train-Block Position States

A train's relationship to a block, its **train-block position state**, is one
of four:

1. **Outside** — the train is elsewhere.
2. **Entering** — the leading end has crossed the boundary, the trailing end
   has not fully entered.
3. **Entered** — the entire train is within the block.
4. **Exiting** — the leading end has crossed the far boundary, the trailing
   end has not fully left.

These states are defined by the train body relative to the block, not by the
sensor. The marker roles Exiting, Entering and Entered name the *event* a marker
triggers, not the ongoing state: the Entered marker fires at the moment the
train transitions from Entering to Entered, and the Exiting marker fires in the
*next leg's* context, as the train leaves the block.

### Canonical Enter Marker

The **canonical enter marker** of a block is the endpoint marker whose
detection means the train body has fully entered the block. Which one it is
depends on travel direction and facing:

| Travel direction       | Facing   | Enter marker |
|------------------------|----------|--------------|
| Aligned with section   | Forward  | B (far end)  |
| Aligned with section   | Backward | A (near end) |
| Against section        | Forward  | A (near end) |
| Against section        | Backward | B (far end)  |

A forward-facing sensor sits on the leading side, so it crosses the far
boundary once the body is inside. A backward-facing sensor sits on the trailing
side, so it crosses the near boundary only after the body has moved deep into
the block.

Every other marker role in a route leg is defined by position relative to the
canonical enter marker. Markers before it in travel order may signal entering or
speed changes; markers after it may signal leaving. Additional markers inside
the block section fall into the correct role by which side of the enter marker
they sit on.

## Trains

A **train** (code: `TrainID`) is a physical or virtual train. Its layout
definition holds its name and hardware configuration; its position, speed and
leg queue are simulation state.

A train's **facing** says whether its sensor sits on the leading or trailing
side relative to its travel direction. Facing determines the canonical enter
marker of every block the train passes.

## Route Legs

A **route leg** is one block-to-block traversal and the atomic unit of train
movement. A train only ever moves across legs that have been committed to its
queue (see Train Control). Deciding *which* legs a train takes is a planning
concern; planning produces legs and plays no part in moving the train.

### Sensor Trajectory and Train Body

A leg's **sensor trajectory** is the path the train's sensor follows from one
block's canonical enter marker to the next. Markers are collected along it,
since the sensor is what detects them.

The train has length: its body extends behind the sensor. A leg therefore also
carries the full sections of its start and target blocks, even though the sensor
traverses only part of them. Locking needs the whole block reserved so the body
fits, not just the tracks the sensor crosses.

### Leg Contents

Each route leg is self-contained and carries:

1. **Start block section** — the full section of the starting block.
2. **Travel section** — the tracks between the start and target blocks, from
   the sensor trajectory.
3. **Target block section** — the full section of the target block.
4. **Markers** — collected along the sensor trajectory, each with a role and a
   speed.

### Marker Roles

A **marker role** is the function a marker serves within one leg: Exiting,
Entering, Entered, or none. Markers are collected along the sensor trajectory,
enter marker to enter marker, and assigned roles by position. A leg has at most
one marker of each role, and may lack one when there are not enough markers.

- Exiting: the first marker in the leg, the canonical enter marker of the
  start block. The train starts here and is exiting the start block.
- Entering: the marker immediately before the Entered marker in travel
  order. The train's leading end is crossing into the target block.
- Entered: the last marker in the leg, the canonical enter marker of the
  target block. The train has fully entered the target block. Anchors leg
  completion and therefore lock release.

A typical leg's marker sequence:

```
[Exiting] → ...no role... → [Entering] → [Entered]
```

The same physical marker is Entered at the end of one leg and Exiting at the
start of the next. Markers between Exiting and Entering have no role; they
correspond to the Outside state, with the train between blocks, and serve only
visual progress interpolation.

### Marker Speed

Each marker in a leg has a **baked-in speed** (code: `TrainSpeed`, one of
Slow, Cruise, Fast), resolved at leg construction: the target speed after
passing this marker, determined by the marker's role.

- Exiting — passthrough speed of the start block for the leg's direction.
- Entering / Entered — passthrough speed of the target block for the
  leg's direction.
- **No role** — Cruise.

Speeds are resolved once when the leg is built, so the driver receives exactly
the speeds it needs without consulting block data.

When a train intends to stop at the end of its current leg, that is, no next leg
is queued, the driver overrides the baked-in speeds: it slows after the Entering
marker and stops after the Entered marker. Baked-in speeds apply only when
passing through.

### Travel Section Markers

- A travel section has zero to many markers.
- When a travel section has no marker near a block boundary, the adjacent
  block's endpoint marker serves as the Exiting or Entering marker. This is
  a conservative fallback: the train stays in transitional states longer, and
  correctness is preserved.
- Extra markers in a travel section help show visual progress and do not change
  block state logic.

### Locking

Locking is a planner concern. Train movement never sees it: a leg in a train's
queue is driven on the assumption that nothing else is in the way, and it is the
planner's job to make that assumption true before committing the leg.

Each leg has a **footprint**, the tracks it needs exclusively: its start block
section, its travel section and its target block section.

The **planner** is the layer that picks the next leg from a plan, acquires its
footprint, and commits it to the leg queue. It owns the **lock table**, the
per-track record of which train holds which track. Before committing a leg, the
planner acquires the leg's whole footprint; a leg whose footprint overlaps
another train's holdings cannot be committed. Legs that share tracks, at every
switch and crossing, conflict through their footprints and through nothing else.

A train holds, at any moment, the section of its position block (see Train
Position) plus the footprints of every leg in its queue. While a train is moving
its holdings are exactly the union of its queued footprints; while it stands
still they are exactly its block section. Placing a train in a block acquires
that block's section; a block held by another train cannot receive a placed
train.

A leg's footprint is held in full until the leg completes: the train has passed
the leg's Entered marker, so its whole body is inside the target block. Only
blocks are guaranteed to contain a whole train, so the Entered marker is the
first point at which the start block and travel section are provably clear. On
completion the leg leaves the queue and the position block becomes its target;
the planner then releases whatever the leg held that is neither the new position
block nor needed by a remaining leg of the same train. Completion depends only on
the Entered marker, so a train that reaches its target with an empty queue, at
its destination or waiting for the planner to acquire the next footprint, frees
the sections behind it and stands in its block until a leg is appended. The
planner learns of completion from the leg removal that movement performs;
movement never calls into the planner.

## Leg Construction

A leg connects two **adjacent** logical blocks: its sensor trajectory is a path
of logical tracks from one block's canonical enter marker to the next, with no
other enter marker in between. Leg construction is local and needs only that
path slice and the block data at both ends:

1. Resolve the full block sections of the start and target blocks.
2. Extract the travel tracks from the path slice.
3. Collect markers along the sensor trajectory.
4. Assign marker roles by position within the leg.

A leg's block sections, markers and speeds depend only on the blocks it
connects, never on a larger path or plan it is part of.

### Leg Graph

All possible legs together form the **leg graph**: its nodes are logical blocks
and its edges are legs. Two nodes may be connected by several edges when parallel
track paths exist between the blocks. The leg graph is derived purely from the
layout and changes only when the layout does.

Planning operates on the leg graph, never on the track graph directly. Train
movement, the leg queue, leg completion and the driver, sees only the legs in a
train's queue and does not depend on where they came from.

## State Events

A **state event** describes a resulting change to simulation state, not its
cause. Simulation logic decides what happens; the event it emits describes the
outcome, so applying an event is a mechanical state update that needs no
knowledge of simulation logic. State events are the only way simulation state
changes. They are keyed by domain IDs (train, block, track) so that they can be
serialized and applied anywhere the state is held.

The state events of the train simulation:

- `PlaceTrain` — set a train's position block; requires an empty leg queue.
- `AppendLegs` — append built route legs to a train's queue.
- `TrainMarkerHit` — the train passed the next marker of its front leg.
- `CompleteLeg` — remove the front leg from the queue and set the position
  block to its target.
- `LockAcquired` / `LockReleased` — a track's lock table entry changed.

## Train Control

Train behavior is driven by a layered model, from low level to high.

### Train Position

Every train on the layout has a **train position**, made of two values:

- Its **position block** — the logical block the train last fully entered.
  **Placement** sets it while the train's leg queue is empty; this is how a
  train enters the layout. From then on it changes only when a leg completes,
  becoming the completed leg's target.
- Its **marker index** — how far the train has progressed along the front leg
  of its queue: the index of the last marker its sensor passed. It is zero
  whenever a leg becomes the front leg, since a leg's first marker is the
  position block's enter marker, which the train has already passed.

The position block is always the start block of the front leg. Appending to an
empty queue requires the leg's start block to equal the position block;
appending to a non-empty queue requires it to equal the last queued leg's
target. The train's state within its leg (Exiting, Outside, Entering, Entered)
is derived from the marker index and the roles of the front leg's markers.

A train with an empty queue is standing still in its position block. A train
with a non-empty queue is executing the front leg.

### Leg Queue

Each train has a **leg queue**: the legs it has committed to and not yet
completed, in traversal order. The **front leg** is the first in the queue, the
one the train is executing. **Leg completion** is the moment the train's sensor
passes the front leg's Entered marker: the leg leaves the queue and the position
block becomes its target. If another leg is queued it is now the front leg and
the train continues without stopping. If the queue is empty the train stands in
its position block until a leg is appended.

Appending a leg to the queue is the **commit**: only the planner commits, and
only once it holds the leg's footprint, so everything in the queue is safe to
traverse. Movement, leg completion and the driver handoff, operates on the
queue alone. It has no notion of locks or of other trains; a leg in the queue is
driven, full stop.

### Plans

A **plan** is the planning layer's view of where a train is going: a subgraph
of the leg graph, rooted at the train's **plan root**, in which every path leads
to the destination. The plan root is the last queued leg's target block, or the
position block when the queue is empty. The planner picks an outgoing edge from
the root, acquires its footprint, and commits it to the leg queue.

A plan may be a single linear path or a graph with alternatives; which one
depends on how the planner searches the leg graph, such as shortest path or all
paths within a cost bound. With alternatives, the train decides dynamically
which leg to commit next: if one branch is held by another train, the planner
commits a different edge toward the same destination without discarding the
plan. Only the committed queue is fixed; the plan beyond it may be revised at
any time.

### Destinations

A **destination** is a target a train wants to reach: a single block, or a set
of acceptable blocks of which any one satisfies it, optionally constrained by
target direction and facing.

A destination drives planning: the planner searches the leg graph from the
train's plan root to the destination and produces a plan. The plan serves the
destination; it can be revised or replaced as long as it still reaches it.

### Strategies

The highest level of control assigns destinations to trains over time. A
**strategy** is a pluggable policy that produces destinations and nothing else:
it never touches plans or legs.

- A **schedule strategy** assigns a fixed, repeating sequence of destinations.
- A **random strategy** assigns arbitrary destinations periodically.
- Further strategies, such as demand-driven or priority-based, fit the same
  slot.

## Train Driver

The **train driver** bridges the simulation and the physical or simulated
train. It receives leg data, executes it autonomously, and reports marker hits
back. The simulation state is hardware-agnostic: it reacts to marker hits
without knowing what produced them, so it behaves identically whether trains are
BLE hardware or virtual.

### Interface

Simulation → driver: append a leg to the driver's queue, carrying the leg's
markers and the train's facing.

Driver → simulation: a **marker hit**, the sole driver-to-simulation signal,
reporting that the train passed a marker.

The driver does not report leg completion; the simulation infers it from the
Entered marker hit. A hardware train advances through its own leg queue locally
while the simulation derives its own state independently from the marker hits.

### Driver Behavior

The driver keeps a queue of legs. Each leg carries its markers, with role, speed
and detection data, and a facing.

1. Execute the current leg, advancing through its markers in order.
2. Let marker metadata determine speed: slow at Entering, stop at Entered when
   no next leg is queued.
3. When the current leg is complete, advance immediately to the next queued
   leg, or stop when there is none.
4. Report each marker hit to the simulation.

The driver's leg data holds only markers and facing: no block sections, block
identities or locks. The planner controls when a train proceeds by controlling
when it commits legs, and dispatch to the driver follows directly from a leg
entering the train's queue. Neither the queue nor the driver knows about locks;
the driver only knows whether it has more legs to execute.

### BLE Driver

The **BLE driver** is backed by a MicroPython train hub that reacts to sensor
detections locally, without a round trip to the simulation. The simulation
downloads committed legs to the hub's queue; each encodes the marker sequence
with expected colors, the marker roles that trigger speed changes, and the
facing.

The hub's onboard logic handles sensor reading and color matching, speed control
from marker roles, autonomous advancement when the next leg is already queued,
and stopping when none is. It reports asynchronously: a marker passed, with its
index, or an unexpected marker, a color mismatch treated as a safety event.

Only committed legs are sent to the hub: a leg on the train is safe to traverse.

### Virtual Driver

The **virtual driver** simulates travel without hardware. It advances a
continuous position against the leg's marker positions at a simulated speed and
emits a marker hit whenever the position crosses the next marker. Marker
positions reflect actual layout distances, so simulated travel times are
consistent across legs of different lengths. It receives the same leg data as
the BLE driver and produces the same events; the simulation does not distinguish
the two.
