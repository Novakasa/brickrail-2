# Property-Based Testing

Use `proptest` to generate random layouts and scenarios, then assert invariants hold. Tests run headless against the core components; failed cases can be replayed visually in the UI from the proptest seed.

## Layers

### Layout invariants

Generate random track/connection/block layouts and verify structural correctness:

- Every connection references two existing tracks with compatible orientations
- Block sections contain valid directed tracks that form a continuous chain
- Logical graph has no dangling edges (every node corresponds to a real track)
- Connection graph is symmetric (if A connects to B, B connects to A)

### Route invariants

Given a valid generated layout, pick random start/target block pairs:

- A* either finds a path or correctly returns None (no panic)
- Built route legs have continuous marker chains (each leg's last marker connects to the next leg's first)
- Start and target blocks on each leg match the path
- The first leg appended to an empty queue starts at the train's position block

### Simulation invariants

Generate a layout, build routes for one or more trains, run the virtual driver for N ticks:

- Train position marker_index never exceeds the current leg's marker count
- Leg state transitions are valid (never goes backwards in the state machine)
- A train with an empty queue never moves; a marker hit while the queue is empty is a safety fault
- Trains on separate non-overlapping routes never interfere with each other
- After enough ticks with a complete route, the train's position block is the target block and its queue is empty

## Visual replay

Proptest provides a seed for every generated case. When a test fails:

1. Grab the seed from the failure output
2. Feed it into the visual client to render the scenario
3. Watch the simulation play out to understand what went wrong

The visual mode is also useful for exploratory testing — run random generation with rendering to build intuition about what invariants should be checked, and to spot issues that aren't yet codified as formal properties.

## Determinism

Headless invariant checks don't require determinism — they assert properties that must hold regardless of execution order or timing.

Visual replay does require determinism. The core components are pure: time reaches the simulation only as the `now` carried on each input, and the virtual driver is advanced by whatever ticks the test passes it. A replay is therefore the recorded input sequence fed back in the same order. Map iteration order and sorting are the remaining sources of non-determinism to keep out of the core.

Controlled time is also useful beyond testing: pause, slow-mo, and frame-by-frame stepping, all of which the host provides by controlling the clock it passes in.

A useful meta-test: run the same seed twice and assert the state-event sequence is identical. This catches non-determinism regressions early, before they manifest as flaky property tests.

## Generators needed

- `TrackID` — random cell positions and orientations
- Connected layout — generate a valid track graph (random walk or grid-based)
- Block placement — pick pairs of tracks in the layout to form blocks
- Marker placement — place markers on tracks (at minimum, on block tracks)
- Train scenarios — pick start/target blocks, build routes, configure driver speed
