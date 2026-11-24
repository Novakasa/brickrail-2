## Orientation

Read `docs/design/` first: `README.md` lists the systems, `architecture.md`
says how core, hosts and UI fit, `layout.md` and `train-simulation.md` define
the domain. Work items are in `docs/STATUS.md`; the stack decisions and their
reasoning are in `docs/rfc/0001-tech-stack-reset.md`.

The core crate (`brickrail-core`) is pure: no I/O, no clock, no async, no
framework. It compiles to native and to `wasm32-unknown-unknown`. Hosts own
all of those concerns and embed the core.

Tools come from the dev shell: `nix develop -c cargo test`.

<!-- driftless v5 -->
## Docs & Decisions (driftless)

This project uses the driftless artifact types — overview in the
`driftless:conventions` skill, per-type specs in `driftless:rfc` /
`driftless:adr` / `driftless:design`:

- **RFC** (`docs/rfc/`) — frozen, dated design-reasoning snapshots.
- **ADR** (`docs/adr/`) — frozen, one decision each (Context / Decision /
  Alternatives). High bar: only genuine architectural decisions.
- **The design** (`docs/design/`: `README.md` root, one file per system)
  — living, high-level description of the systems the developer chose to
  cover; defines their terms. Design changes edit it, code changes never do;
  rewrite, never append.
- **`docs/STATUS.md`** — living per-topic state (one row, one present-tense
  sentence per topic) plus an append-only idea + todo inbox. **Consult it
  when planning; capture todos and ideas there, not in agent memory.** The
  editing verbs are in `driftless:conventions`.

Other documents live freely in `docs/` — driftless defines nothing there.

Freeze rule: only edit what isn't frozen; supersede frozen artifacts, never
rewrite them.

Routing:

- Design reasoning → a draft RFC; its conclusions live in its decision-log.
  Freeze late and light. `/driftless:rfc-session` holds a deliberation and
  lands the RFC.
- A foundational decision in force → an ADR; the altitude gate in
  `driftless:adr` sets the bar.
- What a covered system is, and its vocabulary → the design; otherwise the
  RFC. Ideas, todos, and topic state → `STATUS.md`. A design change goes RFC
  or ADR first, then the design.
<!-- /driftless -->
