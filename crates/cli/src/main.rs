//! vita multicall driver — thin wrapper. Parses argv, dispatches on the
//! `argv[0]` basename (`vita` one-shot vs `vcmp`/`velab`/`vrun` staged stubs),
//! and exits with the pipeline's exit code. All real logic lives in `cli::run`
//! so it is unit-testable without spawning a process.
//!
//! The pipeline contains user-depth-controlled recursion: the recursive-descent
//! parser, and recursive AST walks (`Clone`/`Drop`/elaborate) over deeply nested
//! expressions, sequences, and SVA property trees. The default MAIN-THREAD stack
//! is only ~1 MiB on Windows (vs ~8 MiB on Linux/macOS), so a pathologically deep
//! design overflows it and aborts (SIGABRT / `STATUS_STACK_OVERFLOW`) BEFORE a
//! depth-cap diagnostic (e.g. `SVA_SEQ_ALT_CAP`) can report cleanly. Run the whole
//! driver on a worker thread with a large explicit stack so depth caps produce a
//! clean diagnostic on EVERY OS — the same approach rustc/swc use. This is the
//! sole place the stack is sized; `cli::run` stays in-thread for unit tests, which
//! keeps the work single-threaded and deterministic (just on a bigger stack).
/// Restore the DEFAULT SIGPIPE disposition (Unix). The Rust runtime IGNOREs
/// SIGPIPE at startup, so a write to a pipe whose consumer has closed
/// (`vita design.sv | head`) returns EPIPE, which the `print!`/`println!`
/// machinery turns into a panic (`failed printing to stdout: Broken pipe`, exit
/// 101). Resetting to `SIG_DFL` makes the OS terminate the process on the broken
/// pipe (the conventional producer behaviour, exit 141) — quiet, not a panic.
/// Process-wide for the signal DISPOSITION (not the per-thread MASK): on Linux
/// the worker thread inherits SIG_DFL and dies on the broken-pipe write; on
/// macOS the spawned thread has SIGPIPE masked, so its writes see EPIPE (no
/// signal) — that case is handled by StderrSink's broken-pipe-safe
/// `out_write`/`err_write` (crates/cli/src/lib.rs, the §4.5.59 follow-on).
/// No-op on Windows (no SIGPIPE). A tiny FFI avoids pulling in `libc` for one
/// call; `SIGPIPE` is 13 and `SIG_DFL` is 0 on every Unix target vita builds
/// for (Linux/macOS).
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal(2)` with `SIG_DFL` only resets a signal's disposition to
    // the OS default; it allocates nothing and the ignored return is the prior
    // handler. This is the standard CLI idiom.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    /// 256 MiB — virtual address space (lazily committed), generous headroom over
    /// every OS default. Matches the order of magnitude swc/other Rust compilers use.
    const STACK_SIZE: usize = 256 * 1024 * 1024;

    let argv: Vec<String> = std::env::args().collect();
    let code = std::thread::Builder::new()
        .name("vita-main".to_string())
        .stack_size(STACK_SIZE)
        .spawn(move || cli::run(&argv))
        .expect("spawn vita worker thread")
        .join()
        // A panic in the worker has already printed its message via the default
        // hook; re-raise it on this thread so the process exits with the same
        // conventional panic code (101) as before — NOT a vita exit class
        // (1 user/design error, 2 stale/artifact-gate, 3 CLI error), which would
        // mislead callers. (`spawn` failure above is `.expect()` -> also 101: the
        // 256 MiB is lazily-committed virtual memory so the reservation is cheap
        // and failure is near-impossible; falling back to in-thread would just
        // re-introduce the very ~1 MiB overflow this wrapper removes.)
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    std::process::exit(code);
}
