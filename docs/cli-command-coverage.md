# CLI command CI-coverage matrix

Resolves [#1980](https://github.com/dora-rs/dora/issues/1980).

Maps every source module under [`binaries/cli/src/command/`](../binaries/cli/src/command/)
to the CI that exercises it: PR CI (`.github/workflows/ci.yml`), nightly
(`.github/workflows/nightly.yml`), or neither. This is a *tooling output* —
a snapshot to find untested CLI surface, not a gate. Re-derive it when the
command tree or the workflows change.

This complements [`testing-matrix.md`](testing-matrix.md), which maps by
*capability/area*; this one maps by *command source file*.

## Coverage legend

Each cell records the **strongest** coverage present at that tier:

| Level | Meaning |
|---|---|
| **e2e** | Real execution with a behavior assertion (exit code / output / log marker) |
| **semantic** | Output-asserting smoke step (`validate`/`expand`/`graph`/`completion`) |
| **help** | `<cmd> --help` argparse smoke loop only |
| **parse** | clap argparse unit tests in [`command/mod.rs`](../binaries/cli/src/command/mod.rs) `#[cfg(test)]`, run by `cargo test --all` |
| **—** | no coverage at that tier |

`parse` runs in the ci.yml `test` job **and** is repeated by the nightly
`test-cross-platform` job (macOS + Windows). `help` runs in both the ci.yml
`test` job's argparse loop and the nightly `test-cross-platform` loop — except
`completion`, `inspect`, and `self`, which are in the ci.yml loop only.

The **Behavioral gap** column flags modules that are **never actually run**
anywhere in CI (parse/help don't execute the command's logic).

## Matrix (top-level command modules)

| Module | `dora` command | ci.yml | nightly.yml | Behavioral gap? |
|---|---|---|---|---|
| `run.rs` | `run` | **e2e** (`contract-tests` via `dora run --stop-after`) | **e2e** (`log-sinks`, `service-action`, `streaming`, `cpu-affinity`, `cli-tests`, `examples`) | no |
| `up.rs` | `up` | **e2e** (`e2e`/ws-cli) | **e2e** (`cluster-smoke`, `topic-and-top-smoke`, `cli-tests`) | no |
| `down.rs` | `down`/`destroy` | **e2e** (`e2e`/ws-cli) | **e2e** (`cluster-smoke`, `cli-tests`) | no |
| `build/` | `build` | **e2e** (`e2e`, `fault-tolerance-e2e`, `contract-tests`) | **e2e** (`cli-tests`, `cluster-smoke` deps) | no |
| `start/` | `start` | **e2e** (`e2e`/ws-cli) | **e2e** (`cluster-smoke`, `cli-tests`, `redb`, `state-reconstruction`) | no |
| `stop.rs` | `stop` | **e2e** (`e2e`/ws-cli) | **e2e** (`cluster-smoke`, `cli-tests`) | no |
| `restart.rs` | `restart` | **e2e** (`e2e`/ws-cli) | **e2e** (`topic-and-top-smoke`) | no |
| `list.rs` | `list`/`ps` | **e2e** (`e2e`/ws-cli) | **e2e** (`cluster-smoke`, `cli-tests`) | no |
| `node/` | `node` | **e2e** (`e2e` + `contract-tests`: `node-lifecycle-e2e`) | **help** (`test-cross-platform` loop + parse) | no |
| `param/` | `param` | **e2e** (`e2e`/ws-cli: set/get/list/delete) | **help/e2e** (`topic-and-top-smoke`: `param list`) | no |
| `validate.rs` | `validate` | **semantic** (`test`: `--strict-types` ± mismatch) | **semantic** (`test-cross-platform`) | no |
| `expand.rs` | `expand` | **semantic** (`test`: module flattening) | **semantic** (`test-cross-platform`) | no |
| `graph.rs` | `graph` | **semantic** (`test`: mermaid node IDs) | **semantic** (`test-cross-platform`) | no |
| `completion.rs` | `completion` | **semantic** (`test`: `completion bash`) | — (ci help loop only; no nightly loop / no parse test) | no |
| `new.rs` | `new` | **help** + parse | **e2e** (`cli-tests`: rust/python/c/cxx templates) | no |
| `doctor.rs` | `doctor` | **help** + parse | **e2e** (`topic-and-top-smoke`: `PASS Coordinator`) | no |
| `logs.rs` | `logs` | **help** + parse (12 cases) | **e2e** (`topic-and-top-smoke`) | no |
| `record.rs` | `record` | **help** + parse | **e2e** (`record-replay`) | no |
| `replay.rs` | `replay` | **help** + parse | **e2e** (`record-replay`) | no |
| `trace.rs` | `trace` | **help** + parse (list/view) | **e2e** (`topic-and-top-smoke`: list/view) | no |
| `topic.rs` | `topic` | **help** + parse (list/hz/echo/pub) | **e2e** (`topic-and-top-smoke`: list/info/echo/hz/pub) | no |
| `inspect/` | `inspect`/`top` | **help** + parse (`inspect top`) | **e2e** (`topic-and-top-smoke`: `top --once`, `inspect top --once`) | no |
| `self_.rs` | `self` | **help** (ci loop only) | **e2e** (`topic-and-top-smoke`: `self update --check-only`) | no |
| `hub/` | `hub` | parse | **e2e** (`hub-smoke`: search/info/list/fetch/install-stub/publish/yank/outdated/update) | no |
| `cluster/` | `cluster` | parse (up/status/down/restart) | **e2e** (`cluster-smoke`: status/down; `cluster-e2e`: up via SSH; `ws-cli-e2e`: restart request/reply) | no (see subcommand notes) |
| `coordinator.rs` | `coordinator` (hidden) | — | **e2e** (`redb`, `daemon-reconnect`, `state-reconstruction`) | no |
| `daemon.rs` | `daemon` (hidden) | — | **e2e** (`redb`, `daemon-reconnect`, `state-reconstruction`; also via `dora up`) | no |
| `runtime.rs` | `runtime` (hidden) | — | **e2e** (indirect: `examples` python-operator runtime nodes) | no (indirect only) |
| `node_binary.rs` | *(helper, not a command)* | — | **e2e** (indirect: `record-replay` resolves record/replay node binaries) | no (indirect only) |
| `system/` | `status`/`check`, `system status` | **e2e** (`ws-cli-e2e`: status JSON, missing coordinator/daemon, `system status` parity) | **help** + parse (`status`) | no |
| `clean.rs` | `clean` | **e2e** (`ws-cli-e2e`: table, JSON, quiet, persisted-store error) | parse | no |

## Subcommand-level notes

Some modules with directory granularity have uneven subcommand coverage:

- **`cluster/`** — Behaviorally covered: `up` (`cluster-e2e`, SSH path),
  `status` + `down` (`cluster-smoke`), and `restart` (`ws-cli-e2e`
  subprocess verifies `RestartByName` request and restarted reply handling).
  Unit coverage also checks local remote-command construction for `up`,
  `install`, `uninstall`, and `upgrade`, including systemd unit content and
  SCP port arguments. Full `install`/`uninstall`/`upgrade` execution still
  requires external SSH/systemd targets.
- **`node/`** — Fully covered by `node-lifecycle-e2e` (`add`, `connect`,
  `disconnect`, `info`, `list`, `remove`, `restart`, `stop`) in the ci.yml
  `e2e` job (Rust + C++) and `contract-tests` (Python). `ws-cli-e2e` also
  exercises `info`/`list`/`stop`/`restart`.
- **`param/`** — `set`/`get`/`list`/`delete` all hit in `ws-cli-e2e`;
  `list` additionally in nightly `topic-and-top-smoke`.
- **`topic/`** — `list`/`info`/`echo`/`hz`/`pub` covered in
  `topic-and-top-smoke`. `selector.rs` is internal plumbing (no direct
  command).
- **`hub/`** — `search`, `info`, `list`, `fetch`, the deliberate `install`
  error stub, `publish`, `yank`/`--undo`, `outdated`, and `update` are covered
  by the nightly `hub-smoke` suite using a hermetic local index and source
  repository.
- **`build/`** — submodules `distributed`/`git`/`local`/`lockfile` are
  exercised transitively: `git` via `examples` (`rust-dataflow-git`),
  `local`/`lockfile` via every `dora build`/`dora run`, `distributed` via
  `cluster-e2e` / `multiple-daemons`.
- **`start/attach.rs`** — `dora start` (detach) is hit in many jobs; the
  *attached* (non-`--detach`) path is exercised by `dora run` and the
  ws-cli e2e foreground flows.

## Behavioral gaps (no CI ever runs the command)

No top-level command module currently has only parse/help coverage at the CI
tier. The previous gaps for `clean.rs` and `system/` are covered by
`tests/ws-cli-e2e.rs::clean_status_cli::*`.

Lower-confidence / indirect-only:

1. **`runtime.rs`** and **`node_binary.rs`** are only reached transitively
   (operator runtime nodes; record/replay node-binary resolution). No test
   targets them directly, so a regression isolated to these would only surface
   if the dependent example/job also breaks.

## How this was derived

- Command tree: `binaries/cli/src/command/` + the `Command` enum and
  `#[cfg(test)]` argparse tests in
  [`command/mod.rs`](../binaries/cli/src/command/mod.rs).
- ci.yml: the `test` job's CLI smoke/semantic steps + `e2e` (`ws-cli-e2e`,
  `fault-tolerance-e2e`, `node-lifecycle-e2e` Rust/C++) + `contract-tests`
  (`example-smoke contract_`, `node-lifecycle-e2e` Python).
- nightly.yml: the per-command smoke jobs (`topic-and-top-smoke`,
  `cluster-smoke`, `cluster-e2e`, `record-replay`, `redb-backend-smoke`,
  `daemon-reconnect-smoke`, `state-reconstruction-smoke`, `cpu-affinity-smoke`)
  plus `cli-tests`, `examples`, `test-cross-platform`, `log-sinks`,
  `service-action`, `streaming`.

Note the ci.yml `test`/`e2e`/`contract-tests`/`bench` jobs run on push-to-main
and Trunk merge-queue batches, **not** on every individual PR (see the `if:`
guards in `ci.yml`). The cheap gates (fmt/clippy/check/typos/audit/unwrap/
license) run on every PR.
