# brickrail-2

A layout editor and train-control simulation for LEGO train layouts.

<!-- driftless v5 -->
This directory is **the design**: the living, high-level description of the
system as agreed, which also defines the project's terms. Below, the systems
and how they fit together — one sentence and a link each; each system's design
is a file beside this one. Design changes edit it, code changes never do;
rewrite, never append. Spec: `driftless:design`.
<!-- /driftless -->

## Systems

- **Architecture** — a core of pure components embedded by native and browser
  hosts, one web UI, and the files, modes, commands and events that connect
  them. ([architecture.md](architecture.md))
- **Layout** — the grid of cells and layers, tracks and their orientations,
  connections, and the logical graph that pathfinding runs on.
  ([layout.md](layout.md))
- **Train simulation** — moves trains safely across a layout: markers, blocks
  and legs; the leg queue, locking and planning; the driver that executes legs
  on hardware or virtually. ([train-simulation.md](train-simulation.md))
