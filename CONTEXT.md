# Terms

Project terms whose system has no design file yet. They move into
`docs/design/` as the architecture design file is written.

## Data tiers & modes

**Static layout**:
Tracks, connections, blocks, markers, train definitions, destinations and schedules — everything serialized in the layout file. Frozen while in control mode.

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

## Commands & events

**Command**:
The canonical entry point for any state mutation, whether from the GUI, scripts or property tests. Has a Pending → Completed / Failed lifecycle.

**Layout command**:
A command that modifies the static layout, valid only while the simulation is stopped. Undoable and kept in undo history.

**Control command**:
A command that crosses into simulation territory, valid only while the simulation runs. Not undoable.

**Command response**:
The simulation's reply to a control command, correlated by command ID.

**Transport**:
The layer that moves commands, events and responses between client and simulation. The core never knows which transport is in use.

**Position interpolation**:
Client-side smoothing of train position between discrete state events. The server never sends continuous position.

## Client

**Headless layer**:
Commands, layout state, simulation and mode state — everything testable without rendering or UI.

**Visual layer**:
Widgets, edit buffers, rendering, selection and input mapping. Reads state freely; writes only through commands.

**Application state**:
Whether the app is editing or running a simulation. Lives in the headless layer; transitions are commands.

**Interaction state**:
Active tool, selection and in-progress edit buffers. Purely visual-layer; commands never reference it.

**Edit buffer**:
A widget's working state that accumulates an edit and flushes as a single command on commit.

## Testing

**Property test**:
A `proptest` case that generates random layouts or scenarios and asserts layout, route or simulation invariants.

**Visual replay**:
Re-running a failed property-test seed in the client with controlled time to watch the scenario play out.
