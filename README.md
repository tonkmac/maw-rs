# maw-rs

Rust port of maw-js portable core.

## Phase 1

- Cargo workspace scaffolded.
- `crates/maw-matcher` ports maw-js target normalization and name-resolution logic.
- Rust tests consume the same portable JSON fixtures from maw-js `test/spec/`.

Run:

```bash
cargo test --workspace
```
