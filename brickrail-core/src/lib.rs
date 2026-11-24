//! The brickrail core: pure components with no I/O, no clock and no async,
//! embedded by hosts (daemon, desktop app, browser). See `docs/design/`.

pub mod layout;
pub mod primitives;
pub mod route;
pub mod train_position;
