# Tech stack reset — shared understanding

What four rounds settled, in one place. The decision-log of
[RFC-0001](../0001-tech-stack-reset.md) is the durable record; this is the
summary that closes the interview.

## The shape

Bevy leaves the project. In its place: a **core** library of pure components
and **hosts** that embed it.

The core has no clock, no I/O, no async, and compiles to wasm. Its
components: the layout model with layout commands, validation and undo; the
geometry queries the editor needs (snapping, picking, sections, position
interpolation); the simulation; the state mirror (simulation state plus
apply-event); the virtual driver; the Pybricks codec. The simulation's
boundary is two inputs, commands and marker hits, and two outputs, state
events and driver instructions. It knows no driver implementation and always
sees a static layout. Time reaches it only as `now` on each input; it may ask
to be woken at a deadline. It mutates its own state only through the events it
emits, so clients applying those events hold identical state and run no
behaviour.

Hosts own clocks, I/O and routing. Three exist: the native daemon, the Tauri
app's Rust side (the same host library, in-process, never a sidecar), and the
browser (the core as wasm). Bluetooth runs in-process in the native hosts; no
separable hub bridge. The UI is one TypeScript codebase, React panels over a
Canvas2D layout canvas, running unchanged in the browser and the Tauri
webview, holding interaction state only and never mutating layout data.

## Deployments

Required: the combined native app (first target) and a daemon near the layout
with a remote browser client. Nice-to-have: browser-only virtual mode.
Out of scope but not designed against: browser Web Bluetooth mode, mobile.

The daemon serves the built UI over plain HTTP on a configurable port with
the WebSocket on the same port; LAN only, no auth, no TLS, typed address.
The wire is WebSocket with JSON behind one function; connect is snapshot
then event tail; no event log is persisted and the wire is not versioned.

## Files

No database. The layout is a pretty-printed, versioned JSON file at a
user-chosen path; persistent state is a sibling file. The host running the
simulation owns both, so in the daemon deployment the Pi is the layout's home.

## Workspace

`brickrail-core`, `brickrail-wasm` (cdylib wrapper, no logic),
`brickrail-host` (tokio, btleplug, WebSocket, files, UI serving),
`brickrail-daemon`, `brickrail-app` (Tauri), `web/`. The core never depends
on wasm-bindgen.

## Testing

Unit and property tests on core components carry the weight; a wasm smoke
test; host integration tests over the WebSocket with a virtual-driver
simulation and a fake BLE; little UI testing. Visual replay stays a goal.

## Rejected along the way

Keeping Bevy; Bevy as renderer with a DOM inspector; Spacetime; SQLite; egui;
GPUI; a separable hub bridge; lockstep mirroring; a TypeScript apply-event;
fixed-rate argument-less ticks; hosting the UI apart from the daemon.
