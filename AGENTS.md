<!-- bevy v1 -->
## Bevy architecture (bevy plugin)

Modules make plugin APIs explicit — conventions in the `bevy:conventions`
skill. In short:

- Every plugin gets an owning module (one plugin, or a family shipped
  together) whose `pub` items are the plugin's public API, in both
  directions; systems stay private. Modules without plugins (shared
  vocabulary, helpers) are normal.
- Cross-module coupling goes through pub items only; a missing capability
  is a deliberate surface change at the owning module.
- `pub` on a component grants read *and* write to everyone — weigh that
  when widening a surface; strictly read-only sharing goes through
  events/messages instead.- Nesting is private organization: private submodules, `pub use` at the
  owning module's root; consumers cross only published boundaries. An
  item wanted from internals is a boundary call — re-export at the root,
  or promote the submodule.
- The public surface is the test seam: tests build a minimal `App` with
  the plugin and drive/assert through pub items. An outside test needing
  privates signals a wrong surface — a boundary call, not a reason to
  widen pub.
- When a change widens a module's public surface or adds a cross-module
  dependency, run the boundary call by the user before wiring it — unless
  it's already settled below.

### Project boundary decisions

One line each; only what the code can't show — the why of a contested
call, a deliberate exception, a road not taken.

(none yet)
<!-- /bevy -->

<!-- driftless v3 -->
## Docs & Decisions (driftless)

This project uses the driftless artifact types — overview in the
`driftless:conventions` skill, per-type specs in `driftless:rfc` /
`driftless:adr`:

- **RFC** (`docs/rfc/`) — frozen, dated design-reasoning snapshots.
- **ADR** (`docs/adr/`) — frozen, one decision each (Context / Decision /
  Alternatives). High bar: only genuine architectural decisions.
- **Glossary** (`CONTEXT.md`, repo root) — living, tight definitions.
- **`docs/STATUS.md`** — living per-topic state (shipped / active / blocked /
  next) plus an idea + todo inbox. **Consult it when planning; record todos
  and ideas there, not in agent memory.**

Other documents live freely in `docs/` — driftless defines nothing there.

Freeze rule: only edit what isn't frozen; supersede frozen artifacts, never
rewrite them.

Routing:

- Design reasoning → a draft RFC; its conclusions live in its decision-log.
  Freeze late and light. `/driftless:rfc-session` holds a deliberation and
  lands the RFC.
- A foundational decision in force → an ADR; the altitude gate in
  `driftless:adr` sets the bar.
- Vocabulary → the glossary; ideas, todos, and topic state → `STATUS.md`.
<!-- /driftless -->
