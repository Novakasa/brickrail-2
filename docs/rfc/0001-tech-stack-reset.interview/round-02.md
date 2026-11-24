# Tech stack reset — round 2

Builds on [round 1](round-01.md): the simulation boundary, the required
deployments, a TypeScript web UI, and in-process BLE are settled. This round
settles time, the native shell, the browser's state mirror, and the wire.

### Q5 — Does the simulation need time, and how does it get it?

Narrows [Q1](round-01.md). The simulation's inputs are commands and marker
hits, and it reads no clock. Yet some behavior in the design is
time-dependent: a train waiting at a stop before its next destination, a
schedule strategy assigning destinations on a period. The virtual driver's
need for time is already outside the simulation (its host ticks it).

Four ways to handle time in the simulation:

1. **No time inside.** Waiting and periods are the host's problem: a host
   timer fires and sends a command ("depart now"). Simple core, but "how long
   a train waits at a stop" becomes host logic, duplicated across daemon and
   browser.
2. **`now` on every input, "wake me at t" as an output.** Every command and
   marker hit carries the host's current time; the simulation can return a
   deadline, and the host calls back with a `Tick(now)` input at that time.
   The simulation never reads a clock, tests pass any timestamps they like,
   and waiting stays simulation logic. Simulation time is always wall time.
3. **Variable ticks, `tick(elapsed)`.** The host calls at some rate and
   passes the elapsed time. Replay then depends on the recorded elapsed
   values, and correctness on the host keeping up.
4. **Fixed tick rate, `tick()` with no argument.** A tick is a fixed step of
   simulation time by definition; the simulation counts ticks and the host
   calls `tick()` at roughly that rate. Deterministic: replay only needs the
   order of inputs relative to ticks. Pause, slow motion and stepping are
   trivial, stop ticking or tick faster. The cost is that simulation time and
   wall time drift apart whenever the host cannot keep the rate, which the
   browser guarantees in a background tab and a busy daemon can do too. In
   hardware mode that matters: a ten-second wait at a station stretches, and
   hub marker hits arrive in wall time but are stamped in lagging simulation
   time. Time resolution is also capped at the tick length.

Options 2 and 4 both keep waiting as simulation logic and both replay
deterministically. They differ in what "simulation time" means: wall time
delivered by the host (2), or ticks the host promises to deliver on time (4).
A hybrid is possible, ticks that carry `now`, but that is option 2 with a
polling habit.

Which one?

➡️ Option 2. It is the standard sans-IO shape, and with real hardware in the
loop simulation time should be wall time, not a tick count the host may fall
behind on. The pause and stepping conveniences of option 4 are available in 2
as well, since the host owns the clock it passes in: a paused clock pauses the
simulation.

> option 2
---

### Q6 — Native shell: Tauri, in-process, one UI codebase?

Narrows [Q3](round-01.md) and [Q2](round-01.md). With a TypeScript web UI
settled, the combined native app (first target) needs a shell. The proposal
is Tauri: the web UI in the system webview, the Rust side running the same
code as the daemon, in the same process, so BLE and the simulation are native
and no socket is needed. The alternative within Tauri is a sidecar: the shell
spawns the daemon binary and talks to it over a local socket, which recreates
the separate-process pain from the Godot era in a milder form.

Three things to confirm:

(a) Tauri as the native shell, with the daemon code in-process, not as a
sidecar.

(b) One UI codebase for browser and shell is a hard requirement: nothing in
the UI may exist only for one of them beyond a thin platform layer (file
dialogs, menus).

(c) Keeping browser Web Bluetooth and mobile possible later means: the UI
makes no desktop-only assumptions in its interaction model (pointer events,
not mouse-only; no reliance on hover), and the core, including the Pybricks
codec, stays wasm-compilable. Anything else you want on that list?

➡️ (a) yes, (b) yes, (c) yes, that list is sufficient.

> yeah

---

### Q7 — The browser's state mirror: wasm core, or TypeScript apply?

In the daemon-plus-remote-client deployment the browser holds a copy of
simulation state and applies state events; in edit mode it holds the layout
being edited. Two ways to build the browser side:

1. **The core compiled to wasm.** The core (or a thin wrapper crate) is built
   for `wasm32-unknown-unknown`; `wasm-bindgen` emits the JS glue and, with
   `tsify`, TypeScript declarations from the same serde structs. The browser
   loads it as a module.
2. **TypeScript apply with generated types.** `ts-rs` or similar emits
   TypeScript types from the Rust structs; TypeScript implements apply-event
   and holds the state and the layout as plain objects.

How they differ:

- **Where the model lives.** In 1, layout and simulation state are Rust
  structs everywhere, including edit mode: TypeScript issues layout commands
  into wasm and gets a view back; undo, validation and geometry are Rust. In
  2, the layout under edit is a TypeScript object that Rust first sees on
  entering control mode, and the browser's simulation state is a TypeScript
  object mutated by a hand-written apply.
- **What TypeScript implements.** Both: rendering, input, panels, WebSocket
  glue. Option 2 adds apply-event (small) and all editor geometry: snapping,
  hit-testing, section between two picked tracks, leg previews, layout
  validation. That is where a second implementation grows, since every editor
  feature then needs a Rust version for the server and a TypeScript version
  for the interaction, or a round trip to the server per candidate edit.
- **How drift is caught.** In 2, regenerated types break the TypeScript build
  on shape changes, but not on semantics: if `CompleteLeg` also resets the
  marker index, the TypeScript apply must know; a miss is a slightly wrong
  drawing, not an error. In 1 there is one apply; the new bug class is the
  boundary (serialization mismatches, panics needing a panic hook).
- **Toolchain.** 2 is Node, Vite and a codegen step. 1 adds the wasm32
  target, a `wasm-bindgen` CLI pinned to the crate version, a Vite wasm
  plugin, and a boundary to design coarse. Nix makes the pinning routine.
- **Framework fit.** TypeScript frameworks like plain reactive objects; in 2
  the state is one. In 1 the state sits in wasm memory and TypeScript stores
  a view fetched after each applied batch, one step less idiomatic.
- **What each enables.** Only 1 gives browser-only virtual mode and the
  geometry-in-core rule. 2 is faster to a first screen.
- **Migration.** Starting at 2 and moving to 1 discards the TypeScript apply
  (cheap) and the TypeScript geometry (by then, perhaps not).
- **In the Tauri shell** the webview talks to Rust over IPC either way; under
  1 the core runs twice in one app, natively for the simulation and as wasm
  for the mirror. Expected and fine.

Which, and from the first target or later?

➡️ Option 1 from the start. Not because the module is required: the Tauri app
works under option 2 too, with TypeScript applying events that arrive over
IPC. But the webview cannot call Rust in-process, so the Tauri UI is the same
code as the remote browser client and faces this exact choice on day one;
there is no cheaper moment later. What is left is the migration argument:
starting at 2 discards a cheap TypeScript apply and a TypeScript geometry
layer that will not be cheap by then.

> option 1

---

### Q8 — The wire between daemon and client

In the daemon deployment, and inside the Tauri shell if the webview talks to
the Rust side through the same abstraction, the client and the simulation
exchange: commands in, state events and command responses out, and a snapshot
on connect.

(a) Framing: WebSocket.

(b) Encoding: JSON, readable in a browser's devtools and in logs, versus a
binary serde format (postcard or MessagePack), smaller and faster. At a few
events per second either works; the question is what you want to see when
debugging.

(c) Reconnect: the server sends a full snapshot on connect, then the event
tail; the client replaces its mirror with the snapshot. No event history is
retained server-side beyond what is needed for that. Is that enough, or do
you want a persisted event log for replay and debugging (which would be a
file, per the no-database decision)?

➡️ (a) WebSocket. (b) JSON, with the encoding behind one function so it can
change later. (c) Snapshot plus tail, no persisted log for now; a recording
switch that dumps events to a file is a cheap later addition and belongs in
the inbox, not this RFC.

> sure
