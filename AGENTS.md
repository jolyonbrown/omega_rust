# Working agreements for agents in this repo

- **Read `DESIGN.md` first.** It is the single source of truth for gameplay,
  aesthetics, and architecture. If an instruction conflicts with it, the task
  packet you were given wins; note the conflict in your final summary.
- **Offline builds only:** the sandbox has no network. Dependencies are already
  in the cargo cache. Always build/test with `CARGO_NET_OFFLINE=true`.
  Do **not** add, remove, or bump dependencies in Cargo.toml — if you believe a
  new crate is required, stop and say so in your summary instead.
- Rust 2021+, `cargo fmt` formatting, no `unsafe`, no warnings on
  `cargo build` (fix them, don't `#[allow]` them away without cause).
- **The display-list rule:** all rendering goes through `DisplayList` segments.
  Never call macroquad draw functions for game content directly; never render
  filled shapes or TTF text. Stroke font only.
- **Verify your own work:** `cargo test` must pass, and for anything visible run
  the headless harness (`cargo run -- --headless ...`) and check the PNGs you
  produce actually show what the task asked for before declaring done.
- Commit locally with clear messages (`M1: …`); never push, never touch `gh`.
- Keep modules small and separable: `vector` (display list + sinks), `font`,
  `ship`, `enemies`, `particles`, `game` (states), `audio`, `fx`.
