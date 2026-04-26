# Attributions

s7cmd vendors source code verbatim from two upstream Apache-2.0 crates.
Each vendored file carries a header comment naming its upstream source
and version.

## s3sync (Apache-2.0)
- Author: nidor1998
- Repo: https://github.com/nidor1998/s3sync
- Version pinned: 1.57.1
- Vendored files (under `src/sync_bin/`):
  - `cli/mod.rs` (←  src/bin/s3sync/cli/mod.rs)
  - `cli/ctrl_c_handler.rs` (← src/bin/s3sync/cli/ctrl_c_handler/mod.rs)
  - `cli/indicator.rs` (← src/bin/s3sync/cli/indicator.rs)
  - `cli/ui_config.rs` (← src/bin/s3sync/cli/ui_config.rs)
  - `tracing.rs` (← src/bin/s3sync/tracing/mod.rs)

## s3util-rs (Apache-2.0)
- Author: nidor1998
- Repo: https://github.com/nidor1998/s3util-rs
- Version pinned: 0.2.0
- Vendored files (under `src/util_bin/`):
  - `cli/*.rs` (← src/bin/s3util/cli/*.rs, all 25 files)
  - `tracing_init.rs` (← src/bin/s3util/tracing_init/mod.rs)
- Plus the helpers `start_tracing_if_necessary` and `trace_config_summary`
  vendored from `src/bin/s3util/main.rs` into `src/main.rs`.
