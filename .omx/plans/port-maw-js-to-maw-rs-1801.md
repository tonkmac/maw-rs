# Plan: Continue maw-js → maw-rs Cross-Engine AI Team Port (#1801)

Date: 2026-05-21
Mode: planning only — no maw-js or maw-rs implementation in this slice
Owner/channel: `[m5:mawjs-codex-maw-rs]`, reporting to `mawjs-oracle`
Source issue: https://github.com/Soul-Brews-Studio/maw-js/issues/1801
Related gate: #1798 maw-js line coverage gate is met; Rust continuation may proceed.

## Requirements Summary

Issue #1801 defines maw-rs as both a Rust port of maw-js portable core and a proof-of-work demo for cross-engine AI collaboration coordinated through maw-js. The port must preserve the existing maw-rs continuation state: it is **not** greenfield.

Current repo evidence:

- `README.md:5-8` states the governing rule: deterministic/side-effect-free crates first, using the same maw-js JSON fixture contracts before runtime IO, transports, or CLI commands move over.
- `README.md:22-34` shows Phase 1 already extends beyond the original six crates into identity, bring, split, peer, tmux, hub, feed, auth, xdg, bind, fuzzy, plugin scaffold, and plugin manifest.
- `README.md:43-50` keeps Phase 2 focused on side-effecting transport/runtime adapters while running maw-js and maw-rs side-by-side.
- `README.md:52-56` keeps Phase 3 as a future `clap` CLI, beginning with `ls`, `hey`, `peek`, and target resolution helpers.
- `Cargo.toml:1-14` confirms a single Rust 2021 workspace with `unsafe_code = forbid` and pedantic clippy warnings.
- Local git state on 2026-05-21: `main...origin/main [ahead 37]`; the latest local ship is `7a3fc99 Port maw-rs plugin module symbol imports`.

## Non-Goals for This Planning Slice

- Do not implement maw-js behavior in this turn.
- Do not push or open PRs from this plan-only slice.
- Do not replace maw-js as the default CLI until maw-js and maw-rs command parity is fixture/golden-output proven.
- Do not introduce runtime dependencies before injectable boundaries and test doubles exist.
- Do not restart completed crates or re-port behavior already covered by the local commit stack.

## Acceptance Criteria

1. Every Rust slice starts from the closest maw-js test, fixture, or captured golden output and names that source in tests/docs.
2. The current plugin-manifest slice is stabilized before expanding runtime IO.
3. Phase 2 runtime work begins with fake-backed adapter contracts, not host-dependent tmux/network calls.
4. CLI work begins only after at least one runtime/discovery adapter has deterministic test coverage.
5. Each shipped slice passes:
   - `cargo fmt --all`
   - targeted crate test, e.g. `cargo test -p <crate>`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
6. Each significant local ship is committed with Lore trailers plus `Co-authored-by: OmX <omx@oh-my-codex.dev>` and reported to `mawjs-oracle` via `maw hey` or inbox fallback.
7. The full port is not complete until Phase 1 pure core, Phase 2 runtime adapters, and Phase 3 CLI fast paths have same-test/golden-output parity evidence.

## RALPLAN-DR Summary

### Principles

1. Same-test-first: Rust behavior is accepted only after matching the relevant maw-js fixture/test/golden output.
2. Stabilize before expanding: finish the active plugin-manifest/registry contract before opening broad runtime lanes.
3. Fake boundaries before IO: tmux, filesystem, HTTP/federation, and discovery use injectable test doubles before real side effects.
4. Continuation over bootstrap: preserve shipped maw-rs crates and local commits; do not re-solve completed parity areas.
5. Cross-engine evidence: report decisions, blockers, and completed slices through maw-js coordination.

### Decision Drivers

1. Parity confidence: pick work with strong maw-js test or fixture oracles.
2. Integration leverage: prioritize contracts that unblock multiple runtime/CLI commands.
3. Risk containment: isolate host-specific behavior behind adapters so default CI stays deterministic.

### Viable Options

#### Option A — Stabilize current plugin slice first (recommended immediate lane)

Finish the in-progress plugin manifest/registry runtime boundary before opening new Phase 2 adapters.

Pros:
- Uses the already-active local stack around `maw-plugin-manifest`.
- Locks plugin dispatch/guard behavior needed by future runtime/plugin CLI work.
- Keeps same-test-first cadence small and reviewable.

Cons:
- Does not yet produce visible end-to-end tmux or CLI behavior.
- Requires careful naming so fake-backed dispatch contracts are not mistaken for a completed real JS/WASM runtime.

Immediate targets:
- `invokePlugin` universal CLI metadata/help behavior.
- TS dispatch boundary via injected fake handler.
- WASM file-read/guard boundary via injected fake runner; real WASM instantiate/handle remains a later runtime slice.

#### Option B — Begin Phase 2 adapter contracts now

Design fake-backed contracts for tmux, bring same-session guard, unified inventory/discovery, and federation transport.

Pros:
- Moves maw-rs toward runtime usefulness.
- Can be parallelized by write scope.
- Builds directly toward #1801 integration proof.

Cons:
- Higher design risk if plugin/runtime boundaries are still moving.
- Easy to accidentally couple tests to the developer host unless fakes are mandatory.

Best first runtime candidates:
- `maw-tmux` adapter contract: command runner, parser, pane/window/session operations.
- `maw-bring` same-session guard: current session lookup, target resolution, dispatch decision.
- Unified inventory/discovery: stable JSON shape from the #1805 maw-js cluster.
- Federation send/receive: request/auth decisions using fake HTTP/transport clients.

#### Option C — Start Phase 3 CLI shell early

Add a minimal `maw-rs` `clap` binary and wire completed crates behind subcommands.

Pros:
- Gives a visible demo harness.
- Creates a place for golden-output parity tests.

Cons:
- Risks premature command shape before runtime adapters settle.
- Can hide missing semantics behind shallow wrappers.

Recommended use:
- Defer until Phase 2 has at least tmux/discovery or federation mock-backed parity and a golden-output harness.

## ADR

### Decision

Proceed in three ordered lanes:

1. **Stabilize current slice**: finish the `maw-plugin-manifest` registry/invoke contract already in progress.
2. **Phase 2 fake-backed adapters**: tmux, bring guard, discover/inventory, and federation contracts with fakes before real IO.
3. **Phase 3 CLI**: add `clap` binary and command golden-output tests only after Phase 2 contracts are stable.

### Drivers

- #1801 values a demonstrable, test-driven port rather than a greenfield rewrite.
- maw-rs already has significant local work; the next plan must reduce integration risk, not widen it prematurely.
- Fake-backed runtime contracts are the bridge from pure crates to real CLI behavior without flaky host dependencies.

### Alternatives Considered

- Greenfield maw-rs restart: rejected because existing crates and local commits already shipped parity.
- CLI-first implementation: rejected because command shape should follow proven runtime contracts.
- Runtime-only push before plugin stabilization: rejected because plugin registry/invoke behavior is currently active and blocks future plugin CLI/runtime confidence.

### Consequences

- The immediate work remains narrow, but it prevents runtime lanes from inheriting incomplete plugin semantics.
- Phase 2 can parallelize once fake contracts are explicit and write scopes are separated.
- Phase 3 remains intentionally delayed until golden-output parity can be meaningful.

### Follow-ups

- Keep this plan as the execution ledger for `/goal resume`, `$ralph`, `$team`, or Team + Ultragoal.
- Add or maintain a port matrix once README rows become too dense.
- Report every significant shipped slice to `mawjs-oracle`, including known gaps and commands run.

## Work Plan

### Phase 0 — Baseline and Inventory

1. Confirm local state before execution:
   - `git status --short --branch`
   - `git log --oneline --decorate -25`
2. Record current ahead count from `origin/main`.
3. Re-run baseline gates before high-risk runtime work:
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
4. Keep source mapping from:
   - `maw-rs/README.md:22-34`
   - `maw-js/test/spec/*.fixtures.json`
   - high-value isolated tests under `maw-js/test/isolated/`

Deliverable: updated status artifact or matrix, no behavior change required.

### Phase 1A — Stabilize Current Plugin Slice

Execute as one commit per coherent behavior family.

1. `invokePlugin` contract shell
   - maw-js source/tests: `src/plugin/registry-invoke.ts`, `test/00-registry-invoke-default.test.ts`, `test/isolated/registry-invoke.test.ts`, plugin smoke tests in `test/plugin-manifest.test.ts`.
   - Rust target: `crates/maw-plugin-manifest`.
   - Required parity now:
     - CLI `--version` / `-v` / `-version` only when first arg.
     - CLI help flags anywhere.
     - effective CLI surfaces from declared CLI, TS entry, or WASM path.
     - TS dispatch boundary through injected fake runtime.
     - WASM missing-file/read-error and byte-handoff boundary through injected fake runtime.
   - Explicit gap: real JS/TS dynamic loading and real WASM instantiate/handle execution are not complete in this slice.

2. Plugin runtime follow-up design
   - Decide whether real TS/WASM execution belongs in `maw-plugin-manifest`, a new plugin-runtime crate, or a CLI/runtime crate.
   - Prefer keeping manifest/registry parsing dependency-light; move engines/runners out if dependencies become heavy.

Exit criteria: active plugin registry/invoke behavior has tests, gates pass, commit/report shipped, and remaining runtime execution gap is documented.

### Phase 1B — Pure Core Audit and Closure

1. Audit `maw-identity` coverage for canonical session/node identity fixtures.
2. Audit `maw-fuzzy`, `maw-xdg`, `maw-bind`, and other small pure helpers for unported maw-js isolated tests.
3. Add missing same-test-first coverage only where behavior is not already represented.
4. Document completed/deferred status in README or a port matrix.

Exit criteria: pure helper backlog has either Rust parity coverage or an explicit defer reason.

### Phase 2 — Fake-Backed Runtime Adapter Contracts

Before real side effects, write adapter-level tests with fake dependencies.

1. Tmux adapter contract
   - Build on `maw-tmux` helpers already listed in `README.md:26`.
   - Define command runner trait/fake.
   - Port mocked tmux class/impl tests before real `tmux` execution.
   - Preserve split/attach/kill/send-text safety semantics.

2. Bring same-session guard
   - Build on `maw-bring` fixture base in `README.md:23`.
   - Port maw-js post-#1835/#1836 same-session guard tests.
   - Keep session lookup, current pane/env, and command dispatch injectable.

3. Unified inventory/discovery
   - Build on `maw-peer`, `maw-routing`, `maw-tmux`, and `maw-hub`.
   - Use the #1805 maw-js stable JSON shape as the acceptance oracle.
   - Add fake filesystem/process/network inventory sources before real discovery.

4. Federation transport contract
   - Build on `maw-transport`, `maw-auth`, and `maw-hub`.
   - Start with pure request/auth/routing decisions and fake senders.
   - Add real HTTP/transport client only after deterministic tests pass.

5. Zenoh/stylos spike
   - Keep behind a feature flag or research note until dependency/protocol expectations are documented.
   - Do not add it to default CI until deterministic local tests exist.

Exit criteria: at least tmux plus one discovery/federation path has mock-backed runtime parity tests passing without real host dependencies.

### Phase 3 — CLI Integration

1. Add a `maw-rs` binary crate only after Phase 2 contracts are stable.
2. Use `clap` for parsing while preserving maw-js semantics through tests.
3. Port fast-path commands in this order:
   - `ls` / inventory display
   - `peek` / read-only pane/session inspection
   - `hey` / federation send path
   - target resolution aliases and typo suggestions
4. Create golden-output tests from maw-js captured outputs for each command.
5. Run maw-js and maw-rs side-by-side; do not replace default `maw` until command parity is demonstrated.

Exit criteria: `maw-rs` CLI can run selected commands against fake and controlled local fixtures with output parity.

### Phase 4 — Cross-Engine Review and Handoff

1. Send significant slice reports to `mawjs-oracle`.
2. For larger runtime/CLI milestones, request review from Claude/thClaws counterparts with:
   - changed crates
   - fixture/test source
   - known gaps
   - commands run
3. Track review findings as follow-up issues, plan updates, or next-slice acceptance criteria.

Exit criteria: mixed-engine review has accepted or explicitly deferred each major runtime/CLI area.

## Verification Strategy

### Per-Slice Gate

```bash
cargo fmt --all
cargo test -p <crate>
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### Runtime Gate

- Unit tests use fake runners/filesystems/network clients.
- Integration tests touching host tmux are opt-in or hermetic.
- Golden-output tests compare against maw-js captures.

### Full Port Gate

- All portable fixtures pass in Rust.
- Selected runtime/CLI commands pass same-test or golden-output parity.
- maw-js ↔ maw-rs federation path is tested both directions where possible.
- No known untriaged gaps remain in the port matrix.

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Local stack diverges from origin | Keep ahead count visible; avoid rebases during planning; commit small slices. |
| Fake-backed runtime hides real integration bugs | Add real integration smoke tests after fake contracts pass; keep them opt-in/hermetic until stable. |
| Runtime tests become host/flaky | Fake command/filesystem/network boundaries first; mark real tmux tests opt-in. |
| maw-js behavior changes during port | Pin each slice to source commit/test file; re-run maw-js tests when behavior seems ambiguous. |
| CLI golden outputs become brittle | Normalize environment-specific paths/timestamps and keep maw-js captured fixtures versioned. |
| Cross-engine coordination becomes noise | Report only significant ships, design decisions, and blockers; keep summaries short. |

## Available-Agent-Types Roster

- `explore`: fast repo lookup and file/symbol/test mapping.
- `planner`: sequencing, work breakdown, risk flags.
- `architect`: adapter boundaries, runtime/CLI design review.
- `executor`: implementation slices.
- `test-engineer`: fixture/golden-output strategy and flaky-test control.
- `debugger`: failure isolation and parity mismatches.
- `verifier`: completion evidence and claim validation.
- `code-reviewer`: comprehensive post-slice review.
- `researcher`: external docs/reference lookup if stylos/zenoh or SDK behavior is needed.
- `dependency-expert`: dependency selection/upgrade evaluation for runtime HTTP/zenoh/clap choices.

## Follow-up Staffing Guidance

### Single-owner `$ralph` lane

Use for one sequential slice at a time, especially Phase 1A plugin stabilization.

Recommended prompt:

```text
/goal resume with plan .omx/plans/port-maw-js-to-maw-rs-1801.md; execute Phase 1A invokePlugin contract only, verify with targeted + workspace gates, commit locally with Lore trailers and Co-authored-by OmX, and report to mawjs-oracle. Do not push.
```

Suggested roles:
- `explore` to map the maw-js test/source for the selected slice.
- `executor` to implement.
- `verifier` to run gates and validate claim.

### Coordinated `$team` lane

Use when Phase 2 begins, because tmux, bring guard, inventory/discovery, and federation can be designed/tested in parallel with separate write scopes.

Launch hint:

```bash
omx team create maw-rs-port-phase2
omx team send maw-rs-port-phase2 "Use .omx/plans/port-maw-js-to-maw-rs-1801.md. Plan-only first: assign tmux adapter, bring same-session guard, unified inventory/discovery, and federation lanes with fake-boundary tests before implementation."
```

Suggested lanes:
- Architect lane: runtime adapter traits and crate boundaries.
- Test-engineer lane: fake/golden-output contracts from maw-js tests.
- Executor lane A: tmux adapter, write scope `crates/maw-tmux` only.
- Executor lane B: bring/session guard, write scope `crates/maw-bring` plus any new session helper crate.
- Executor lane C: inventory/discovery, write scope new or existing discovery/peer crate.
- Executor lane D: federation transport, write scope `crates/maw-transport`, `crates/maw-auth`, `crates/maw-hub` as agreed by architect.
- Verifier lane: workspace gates and parity checklist.

Team verification path:
1. Team proves each lane has tests that fail before implementation or are directly copied from maw-js fixtures/golden outputs.
2. Team proves no lane requires real host tmux/network for default CI.
3. Ralph or single-owner verifier runs final workspace test/clippy and prepares commit/report.

## Goal-Mode Follow-up Suggestions

- Use `$ultragoal` for durable tracking of the full #1801 port across many slices.
- Use Team + Ultragoal for Phase 2/3: Team handles parallel lane execution; Ultragoal owns the ledger/checkpoints.
- Use `$ralph` only for persistent sequential pressure on a specific slice or post-team verification/fix loop.
- Use `$autoresearch-goal` only if the next blocker is external protocol/dependency research, e.g. stylos/zenoh details.
- Use `$performance-goal` only after maw-rs has measurable runtime paths and a clear performance evaluator.

## Immediate Next Slice Recommendation

Start with **Phase 1A: `invokePlugin` contract shell** in `maw-plugin-manifest`, because it stabilizes the active local plugin stack before Phase 2 runtime expansion. Stop condition for the next execution slice: one local commit, all gates green, status report sent, no push.
