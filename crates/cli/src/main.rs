//! vita multicall driver — thin wrapper. Parses argv, dispatches on the
//! `argv[0]` basename (`vita` one-shot vs `vcmp`/`velab`/`vrun` staged stubs),
//! and exits with the pipeline's exit code. All real logic lives in `cli::run`
//! so it is unit-testable without spawning a process; the shared process-level
//! plumbing (SIGPIPE reset + big-stack worker thread) lives in
//! [`cli::driver_main`] so the dev-only `separate-bins` shims
//! (`src/bin/{vcmp,velab,vrun}.rs`, doc-03) share the exact same code path.
fn main() {
    cli::driver_main()
}
