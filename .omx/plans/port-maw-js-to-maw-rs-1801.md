# Plan: Continue maw-js → maw-rs Cross-Engine AI Team Port (#1801)

Date: 2026-05-20
Mode: planning only — no implementation in this slice
Owner/channel: `[m5:mawjs-codex-maw-rs]`, reporting to `mawjs-oracle`
Source issue: https://github.com/Soul-Brews-Studio/maw-js/issues/1801

## Requirements Summary

Issue #1801 frames maw-rs as both a Rust port of maw-js portable core and a proof-of-work demo for cross-engine AI collaboration coordinated through maw-js. The proposed flow is: Claude/spec team reads maw-js source and fixtures, Codex implements Rust from specs/fixtures, mixed engines review, then maw-rs integrates runtime transports and CLI behavior.

Current repo state confirms maw-rs is a continuation, not a bootstrap:

- `README.md:5-8` states the governing rule: start with deterministic, side-effect-free crates and pass the same maw-js JSON fixture contracts before moving runtime IO, transports, or CLI commands.
- `README.md:16-31` lists already-ported crates through `maw-bind`, including matcher, calver, policy, worktree, transport, routing, identity, bring, split, peer, tmux, hub, feed, auth, xdg, and bind.
- `README.md:40-47` already names Phase 2 as side-effecting transport/runtime adapters while preserving side-by-side maw-js/maw-rs operation.
- `README.md:49-53` names Phase 3 as a future CLI with `clap`, starting from `ls`, `hey`, `peek`, and target resolution helpers.
- `Cargo.toml:1-14` shows a single Cargo workspace, Rust 2021, BUSL-1.1, `unsafe_code = forbid`, and pedantic clippy warnings.
- Local git history shows `origin/main` at `0e4c36c` and local continuation commits through `5213184`, so future work must build on the existing 18-commit local stack rather than restarting.

## Non-Goals for This Planning Slice

- Do not implement maw-js behavior in this turn.
- Do not push or open PRs from this plan-only slice.
- Do not replace maw-js as the default CLI until maw-js and maw-rs command parity is fixture/golden-output proven.
- Do not introduce runtime dependencies before injectable boundaries and test doubles exist.

## Acceptance Criteria

1. Every new Rust slice starts from the closest maw-js test or fixture and names it in the crate docs/tests.
2. Pure-function slices remain dependency-light and deterministic.
3. IO/runtime slices expose injectable command/filesystem/network boundaries before touching real tmux, HTTP, zenoh/stylos, or shell execution.
4. Each slice passes:
   - targeted crate test, e.g. `cargo test -p <crate>`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
5. Each shipped slice updates `README.md` crate/status rows when a new crate or parity area is added.
6. Each significant local ship is committed with Lore trailers and reported to `mawjs-oracle` via the mawjs-codex federation pattern or inbox fallback.
7. The full port is not considered complete until Phase 1 pure core, Phase 2 runtime adapters, and Phase 3 CLI fast paths have same-test/golden-output parity evidence.

## RALPLAN-DR Summary

### Principles

1. Same-test-first: Rust behavior is accepted only after matching the relevant maw-js fixture/test.
2. Fixtures before IO: pure contracts and injectable adapters precede side-effecting integrations.
3. Continuation over bootstrap: preserve shipped maw-rs crates and local commits; do not re-solve completed parity areas.
4. Small, reviewable slices: one behavior family per crate/commit unless fixtures are inseparable.
5. Cross-engine evidence: report decisions, blockers, and completed slices through maw-js coordination.

### Decision Drivers

1. Parity confidence: port the behavior with the best existing maw-js test oracle first.
2. Integration leverage: prioritize helpers that unblock multiple CLI/runtime commands.
3. Risk containment: keep tmux/network/filesystem side effects behind explicit adapters and mocks.

### Viable Options

#### Option A — Continue pure-function backlog first (recommended immediate lane)

Port remaining small deterministic helpers before expanding runtime IO.

Pros:
- Low regression risk.
- Fast commits with clear same-test parity.
- Builds shared utility surface for future CLI ports.

Cons:
- Does not immediately prove end-to-end maw-rs runtime behavior.
- Can defer hard IO problems too long if not time-boxed.

Immediate candidates:
- `maw-fuzzy`: `src/core/util/fuzzy.ts` + `test/fuzzy-match.test.ts`.
- Additional canonical/session helpers only if not already covered by `maw-identity`.
- Small resolver/preset/config pure helpers discovered from maw-js isolated tests.

#### Option B — Move directly into Phase 2 runtime IO

Implement tmux/HTTP/discovery adapters now, building on `maw-tmux`, `maw-transport`, and `maw-hub`.

Pros:
- Higher visible progress toward maw-rs as a real CLI/runtime.
- Exercises earlier pure crates under realistic orchestration flows.

Cons:
- More flakiness risk from tmux/session environment.
- Requires stronger mock/fake boundaries before implementation.
- Harder to review in small same-test slices.

Best first runtime candidates:
- Full tmux adapter from existing parser/safety/action helpers.
- `bring` same-session guard using maw-js post-#1835/#1836 tests.
- Unified inventory/discovery JSON shape from the #1805 cluster.

#### Option C — Start Phase 3 CLI shell early

Add a minimal `maw-rs` `clap` binary and wire completed crates behind subcommands.

Pros:
- Provides an integration harness and visible demo path.
- Lets `ls`, `hey`, and `peek` golden outputs emerge incrementally.

Cons:
- Risks premature public API shape before runtime adapters settle.
- Can produce shallow command wrappers without enough parity coverage.

Recommended use:
- Start only after at least one Phase 2 runtime adapter has mock-backed parity and an agreed CLI golden-output test harness.

## Decision / ADR

### Decision

Proceed in a two-lane plan:

1. Immediate lane: finish remaining pure-function ports in short same-test-first commits.
2. Parallel planning lane: design Phase 2 injectable runtime boundaries for tmux, same-session bring guard, and unified inventory/discovery before implementing IO.

### Drivers

- Same-test parity is the strongest correctness oracle available.
- maw-rs already has a substantial pure-core base; the next work should reuse it instead of restarting.
- Runtime behavior needs careful boundaries to avoid flaky tests and host-specific failures.

### Alternatives Considered

- Greenfield maw-rs rewrite: rejected because existing crates and commits already shipped core parity.
- CLI-first implementation: rejected for now because it would couple public command shape to incomplete runtime adapters.
- Runtime-only push: rejected as the sole next step because small pure utilities remain cheap and unblock later CLI ergonomics.

### Consequences

- The next several commits may look small, but they increase parity coverage and utility reuse.
- Phase 2 must explicitly define adapter traits/fakes before real tmux/network calls.
- Progress reporting remains important because #1801 is partly about cross-engine coordination evidence.

### Follow-ups

- Convert this plan into an execution checklist before resuming `/goal` or `$team`.
- Keep a parity matrix in `README.md` or a dedicated `docs/port-matrix.md` once the crate table becomes too dense.
- When a runtime lane begins, add fixture/golden-output files before side-effecting code.

## Work Plan

### Phase 0 — Baseline and Inventory

1. Confirm local branch/commit state before any execution:
   - `git status --short`
   - `git log --oneline --decorate -25`
2. Record current ahead count from `origin/main`.
3. Re-run baseline gates before high-risk runtime work:
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
4. Build or update a port matrix from:
   - `maw-rs/README.md:16-31`
   - `maw-js/test/spec/*.fixtures.json`
   - high-value isolated tests under `maw-js/test/isolated/`

Deliverable: updated planning/status artifact, no code behavior change required.

### Phase 1 — Pure Core Continuation

Execute as one crate/commit per bullet unless tests prove inseparable.

1. `maw-fuzzy`
   - maw-js source/test: `src/core/util/fuzzy.ts`, `test/fuzzy-match.test.ts`.
   - Rust API: `distance`, `fuzzy_match`.
   - Required parity: Levenshtein distance, empty/exact cases, case-insensitive matching, deduplication, max-distance filtering, max-results truncation, distance/name sorting.
   - Verification: `cargo test -p maw-fuzzy`, workspace tests, clippy.

2. Remaining canonical/session pure helpers audit
   - Verify whether `maw-identity` fully covers `canonical-session-name` and `canonical-node-identity` fixtures.
   - If gaps exist, add tests first, then implementation.
   - If no gaps exist, document as complete in the port matrix rather than duplicating code.

3. Resolver/config/preset pure helpers
   - Mine maw-js tests for pure deterministic helpers around instance preset selection, config parsing, home/path resolution, and command suggestion behavior.
   - Only promote helpers with stable tests or fixtures.

Exit criteria: pure helper backlog has either a Rust crate/test or an explicit “defer to runtime/CLI” note.

### Phase 2 — Runtime Adapter Design, Then Implementation

Before implementation, write adapter-level test contracts with fake dependencies.

1. Tmux runtime adapter
   - Build on `maw-tmux` helpers already listed in `README.md:26`.
   - Define injectable command runner abstraction.
   - Port mocked tmux class/impl tests before real tmux execution.
   - Preserve safety gates and split/attach/kill/send-text semantics already ported in local commits.

2. Bring same-session guard
   - Build on `maw-bring` fixture base in `README.md:23`.
   - Port maw-js post-#1835/#1836 tests for same-session self/foreign safeguards.
   - Keep side effects injectable: session lookup, env/current pane, command dispatch.

3. Unified inventory/discovery
   - Build on `maw-peer`, `maw-routing`, `maw-tmux`, and `maw-hub`.
   - Use maw-js #1805-era stable JSON shape as the acceptance oracle.
   - Add fake filesystem/process/network inventory sources before real discovery.

4. HTTP federation runtime wiring
   - Build on `maw-transport`, `maw-auth`, and `maw-hub`.
   - Runtime client can come after pure request/response/auth decisions are already proven.

5. Zenoh/stylos transport spike
   - Keep behind a feature flag until dependency and protocol expectations are documented.
   - Do not make it part of the default gate until deterministic local tests exist.

Exit criteria: at least tmux and one federation/discovery path have mock-backed runtime parity tests passing without real host dependencies.

### Phase 3 — CLI Integration

1. Add a `maw-rs` binary crate only after Phase 2 boundaries are stable.
2. Use `clap` for command parsing, preserving maw-js command semantics through tests.
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
3. Track review findings as follow-up issues or plan updates.

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
- Golden output tests compare against maw-js captures.

### Full Port Gate

- All portable fixtures pass in Rust.
- Selected runtime/CLI commands pass same-test or golden-output parity.
- maw-js ↔ maw-rs federation path is tested both directions where possible.
- No known untriaged gaps in the port matrix.

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Existing local commits diverge from origin | Keep ahead count visible; avoid rebases during planning; commit small slices. |
| Runtime tests become host/flaky | Fake command/filesystem/network boundaries first; mark real tmux tests opt-in. |
| maw-js behavior changes during port | Pin each slice to source commit/test file; re-run maw-js tests when behavior seems ambiguous. |
| Fixture coverage misses user-visible CLI behavior | Add golden-output tests from maw-js before CLI replacement. |
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

Use for one sequential slice at a time, especially pure-function crates.

Recommended prompt:

```text
/goal resume with plan .omx/plans/port-maw-js-to-maw-rs-1801.md; execute the next same-test-first pure slice only, verify with targeted + workspace gates, commit locally with Lore trailers, and report to mawjs-oracle. Do not push.
```

Suggested roles:
- `explore` to map the maw-js test/source for the selected slice.
- `executor` to implement.
- `verifier` to run gates and validate claim.

### Coordinated `$team` lane

Use when Phase 2 begins, because tmux, bring guard, and inventory/discovery can be designed/tested in parallel with separate write scopes.

Launch hint:

```bash
omx team create maw-rs-port-phase2
omx team send maw-rs-port-phase2 "Use .omx/plans/port-maw-js-to-maw-rs-1801.md. Plan-only first: assign tmux adapter, bring same-session guard, and unified inventory/discovery lanes with fake-boundary tests before implementation."
```

Suggested lanes:
- Architect lane: runtime adapter traits and crate boundaries.
- Test-engineer lane: mock/golden-output contracts from maw-js tests.
- Executor lane A: tmux adapter, write scope `crates/maw-tmux` only.
- Executor lane B: bring/session guard, write scope `crates/maw-bring` plus any new session helper crate.
- Executor lane C: inventory/discovery, write scope new or existing discovery/peer crate.
- Verifier lane: workspace gates and parity checklist.

Team verification path:
1. Team proves each lane has tests that fail before implementation or are directly copied from maw-js fixtures.
2. Team proves no lane requires real host tmux/network for default CI.
3. Ralph or single-owner verifier runs final workspace test/clippy and prepares commit/report.

## Goal-Mode Follow-up Suggestions

- Use `$ultragoal` for durable tracking of the full #1801 port across many slices.
- Use Team + Ultragoal for Phase 2/3: Team handles parallel lane execution; Ultragoal owns the ledger/checkpoints.
- Use `$ralph` only for persistent sequential pressure on a specific slice or post-team verification/fix loop.
- Use `$autoresearch-goal` only if the next blocker is external protocol/dependency research, e.g. stylos/zenoh details.
- Use `$performance-goal` only after maw-rs has measurable runtime paths and a clear performance evaluator.

## Immediate Next Slice Recommendation

Start with `maw-fuzzy` because it is pure, small, and already has maw-js tests. This keeps the plan aligned with #1801's fixture/test-driven porting while preserving Phase 2 design time for higher-risk runtime IO.

Stop condition for the next execution slice: one local commit, all gates green, status report sent, no push.
