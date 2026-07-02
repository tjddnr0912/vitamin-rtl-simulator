//! `vita design.sv | head` must NOT panic (report §6 C1). The Rust runtime
//! ignores SIGPIPE, so a write to a pipe whose consumer has closed returns EPIPE
//! and the `print!`/`println!` machinery panics ("failed printing to stdout:
//! Broken pipe", exit 101). `main` restores the default SIGPIPE disposition, so a
//! broken pipe terminates the process via SIGPIPE (signal 13, the conventional
//! producer behaviour) — quiet, not a panic. Unix-only (Windows has no SIGPIPE).
#![cfg(unix)]
use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

#[test]
fn no_panic_when_consumer_closes_early() {
    // Far more output than a pipe buffer (~64 KiB), so the producer is still
    // writing when the consumer closes its read end.
    let src = "module tb; initial begin\n\
               for (int i=0;i<40000;i++) $display(\"line %0d xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\", i);\n\
               $finish; end endmodule";
    let dir = std::env::temp_dir().join(format!("vita_sigpipe_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("t.sv");
    std::fs::write(&f, src).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&f)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vita");

    // Read a little, then drop the read end (close the consumer side of the pipe).
    {
        let mut out = child.stdout.take().unwrap();
        let mut buf = [0u8; 64];
        let _ = out.read(&mut buf);
        // `out` dropped here → the read fd closes → the producer's next full-buffer
        // write gets SIGPIPE.
    }

    let mut err = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut err);
    }
    let status = child.wait().expect("wait vita");

    // The regression: vita must NOT panic on the broken pipe (panic = exit 101).
    assert_ne!(
        status.code(),
        Some(101),
        "vita panicked on a broken pipe (exit 101)\nstderr:\n{err}"
    );
    assert!(
        !err.contains("panicked") && !err.contains("Broken pipe"),
        "vita printed a panic / broken-pipe message:\n{err}"
    );
    // The conventional outcome is termination by SIGPIPE (signal 13). (If vita
    // happened to finish all writes first it would exit normally — also fine; the
    // invariant under test is "no panic".)
    if let Some(sig) = status.signal() {
        assert_eq!(
            sig, 13,
            "killed by an unexpected signal (expected SIGPIPE=13)"
        );
    }
}
