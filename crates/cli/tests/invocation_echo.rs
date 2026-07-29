//! `-v` effective-invocation echo, and the filelist flag-value bug it exposed.
//!
//! The motivating case is a Makefile-driven run: by the time `vita` starts, the
//! shell has already substituted every `$(VAR)`, the `-f` frames have not been
//! spliced yet, and `VITA_THREADS` is nowhere in argv. The transcript has to
//! carry the RESOLVED form or a failing CI job cannot be diagnosed from its log
//! alone. These tests pin what the echo must state and that `--log` captures it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Per-test directory — these tests `cd` the child process and write filelists,
/// so a shared temp dir would race under the parallel test harness.
fn tmpdir(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inv_{}_{n}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita_in(dir: &std::path::Path, args: &[&str], env: &[(&str, &str)]) -> (String, String, i32) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.current_dir(dir).args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// The value of row `label:` as a whitespace-split token list (rows are
/// column-aligned, and long ones wrap onto indented continuation lines).
fn row<'a>(out: &'a str, label: &str) -> Vec<&'a str> {
    let mut it = out.lines().skip_while(|l| !l.starts_with(label));
    let Some(first) = it.next() else {
        return Vec::new();
    };
    let mut toks: Vec<&str> = first[label.len()..].split_whitespace().collect();
    // Continuation lines are pure indentation followed by more values.
    for l in it {
        if !l.starts_with("    ") {
            break;
        }
        toks.extend(l.split_whitespace());
    }
    toks
}

const DESIGN: &str = r#"module t;
  int seed;
  initial begin
`ifdef FAST_MODE
    $display("W=%0d", `W);
`endif
    if ($value$plusargs("SEED=%d", seed)) $display("seed=%0d", seed);
  end
endmodule
"#;

#[test]
fn the_echo_states_the_resolved_form_of_every_scattered_input() {
    let d = tmpdir("resolved");
    std::fs::write(d.join("t.sv"), DESIGN).unwrap();
    std::fs::create_dir_all(d.join("inc")).unwrap();
    // A `.f` whose paths and macro values come from the environment — the
    // Makefile shape. None of these strings appear in argv.
    std::fs::write(
        d.join("build.f"),
        "+incdir+$(INC_DIR)\n+define+FAST_MODE+W=$(WIDTH)\n$(RTL)/t.sv\n",
    )
    .unwrap();
    let (out, _, rc) = vita_in(
        &d,
        &["-f", "build.f", "-o", "w.vcd", "+SEED=7", "-v"],
        &[
            ("INC_DIR", "inc"),
            ("WIDTH", "32"),
            ("RTL", "."),
            ("VITA_THREADS", "3"),
        ],
    );
    assert_eq!(rc, 0, "got:\n{out}");
    // The macro the design compiled with — the whole point. `W=$(WIDTH)` in
    // the .f, `W=32` here.
    assert!(row(&out, "defines:").contains(&"W=32"), "got:\n{out}");
    assert!(row(&out, "defines:").contains(&"FAST_MODE"), "got:\n{out}");
    // The runtime plusarg, which no other line of the transcript mentions.
    assert!(row(&out, "plusargs:").contains(&"+SEED=7"), "got:\n{out}");
    // The filelist that contributed them, and the source it pulled in.
    assert!(
        row(&out, "filelists:")
            .iter()
            .any(|t| t.ends_with("build.f")),
        "got:\n{out}"
    );
    assert!(
        row(&out, "sources:").iter().any(|t| t.ends_with("t.sv")),
        "got:\n{out}"
    );
    assert!(row(&out, "output:").contains(&"w.vcd"), "got:\n{out}");
    // The command as typed, replayable.
    assert!(
        out.lines()
            .any(|l| l.starts_with("invocation:") && l.contains("-f build.f")),
        "got:\n{out}"
    );
    assert!(out.contains("cwd:"), "got:\n{out}");
    // And the design really did see those values.
    assert!(out.contains("W=32"), "got:\n{out}");
    assert!(out.contains("seed=7"), "got:\n{out}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_env_only_knob_is_attributed_to_the_environment() {
    // `VITA_THREADS` never appears in argv; without the echo, "why is this run
    // using 3 threads" is unanswerable from the transcript.
    let d = tmpdir("env");
    std::fs::write(d.join("t.sv"), DESIGN).unwrap();
    let (out, _, rc) = vita_in(&d, &["t.sv", "-v"], &[("VITA_THREADS", "3")]);
    assert_eq!(rc, 0, "got:\n{out}");
    assert_eq!(
        row(&out, "threads:"),
        ["3", "(VITA_THREADS)"],
        "got:\n{out}"
    );
    assert!(row(&out, "env:").contains(&"VITA_THREADS=3"), "got:\n{out}");
    // The flag wins over the env, and says so.
    let (out2, _, _) = vita_in(&d, &["t.sv", "-v", "-j", "2"], &[("VITA_THREADS", "3")]);
    assert_eq!(row(&out2, "threads:"), ["2", "(--threads)"], "got:\n{out2}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_log_tee_captures_the_whole_block() {
    // The reason the echo is a Progress event and not an eprintln: `--log` must
    // hold it, in emission order, ahead of the diagnostics it explains.
    let d = tmpdir("log");
    std::fs::write(d.join("t.sv"), DESIGN).unwrap();
    let (_, _, rc) = vita_in(
        &d,
        &["t.sv", "-v", "-D", "FAST_MODE", "-D", "W=8", "-l", "r.log"],
        &[],
    );
    assert_eq!(rc, 0);
    let log = std::fs::read_to_string(d.join("r.log")).expect("log must exist");
    assert!(log.contains("invocation:"), "log:\n{log}");
    assert!(row(&log, "defines:").contains(&"W=8"), "log:\n{log}");
    assert!(row(&log, "log:").contains(&"r.log"), "log:\n{log}");
    // …and it precedes the RTL output it describes.
    let (i, j) = (log.find("invocation:"), log.find("W=8\n"));
    assert!(i < j && i.is_some() && j.is_some(), "log:\n{log}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn without_v_the_echo_is_absent() {
    let d = tmpdir("quiet");
    std::fs::write(d.join("t.sv"), DESIGN).unwrap();
    let (out, _, rc) = vita_in(&d, &["t.sv", "-D", "FAST_MODE", "-D", "W=1"], &[]);
    assert_eq!(rc, 0, "got:\n{out}");
    assert!(!out.contains("invocation:"), "got:\n{out}");
    assert!(!out.contains("defines:"), "got:\n{out}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_stage_of_the_staged_flow_echoes_its_own_inputs() {
    let d = tmpdir("staged");
    std::fs::write(d.join("t.sv"), DESIGN).unwrap();
    let (c, _, rc) = vita_in(
        &d,
        &["vcmp", "t.sv", "-o", "t.vu", "-v", "+define+FAST_MODE+W=5"],
        &[],
    );
    assert_eq!(rc, 0, "got:\n{c}");
    assert!(row(&c, "defines:").contains(&"W=5"), "got:\n{c}");
    assert!(row(&c, "output:").contains(&"t.vu"), "got:\n{c}");

    let (e, _, rc) = vita_in(&d, &["velab", "t.vu", "-o", "t.velab", "-v"], &[]);
    assert_eq!(rc, 0, "got:\n{e}");
    assert!(row(&e, "sources:").contains(&"t.vu"), "got:\n{e}");
    assert!(row(&e, "output:").contains(&"t.velab"), "got:\n{e}");
    // velab has no preprocess pass — it must not claim a define surface.
    assert!(row(&e, "defines:").is_empty(), "got:\n{e}");

    let (r, _, rc) = vita_in(
        &d,
        &["vrun", "t.velab", "-o", "t.vcd", "+SEED=4", "-v"],
        &[],
    );
    assert_eq!(rc, 0, "got:\n{r}");
    assert!(row(&r, "plusargs:").contains(&"+SEED=4"), "got:\n{r}");
    assert!(r.contains("seed=4"), "got:\n{r}");
    let _ = std::fs::remove_dir_all(&d);
}

// ── the bug the echo exposed: flag VALUES inside a `-F` frame ────────────────

#[test]
fn a_flag_value_in_a_filelist_is_not_a_path() {
    // `takes_value` listed only the original five flags, so inside a `-F` frame
    // (paths anchor to the .f's own directory) every later flag's value was
    // rewritten as if it were a source file: `--top top` became
    // `--top /abs/ip/top` and the run died with "top module not found".
    let d = tmpdir("flagval");
    std::fs::create_dir_all(d.join("ip")).unwrap();
    std::fs::write(
        d.join("ip/t.sv"),
        "module top; initial $display(\"ran\"); endmodule\n",
    )
    .unwrap();
    std::fs::write(d.join("ip/build.f"), "--top top\nt.sv\n").unwrap();
    let (out, err, rc) = vita_in(&d, &["-F", "ip/build.f", "-v"], &[]);
    assert_eq!(rc, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("ran"), "got:\n{out}");
    assert_eq!(row(&out, "tops:"), ["top"], "got:\n{out}");
    // The source positional in the same frame DOES resolve — that is the whole
    // point of `-F`, and the fix must not have disabled it.
    assert!(
        row(&out, "sources:")
            .iter()
            .any(|t| t.ends_with("ip/t.sv") && t.starts_with('/')),
        "got:\n{out}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_filelist_frame_behaves_exactly_like_the_command_line() {
    // The durable guard. `takes_value` is a hand-maintained flag list, and a
    // hand-maintained list rots the moment someone adds a flag without touching
    // it — which is precisely how this bug arrived. So do not re-assert the list:
    // assert the PROPERTY it exists for. The same flags given on argv and given
    // inside a `-F` frame must produce the same output, the same files, in the
    // same places. Any future value-flag omitted from `takes_value` breaks this
    // without anyone having to remember it.
    let d = tmpdir("argvparity");
    std::fs::create_dir_all(d.join("ip")).unwrap();
    std::fs::write(
        d.join("ip/t.sv"),
        "module top; initial $display(\"ran\"); endmodule\n",
    )
    .unwrap();
    #[rustfmt::skip]
    const FLAGS: &[&str] = &[
        "--top", "top",
        "--hier-tree", "h.txt",
        "--inst-paths", "i.txt",
        "--timeout", "100",
        "--threads", "2",
        "-D", "W=8",
        "-I", "ip",
    ];
    std::fs::write(
        d.join("ip/all.f"),
        FLAGS
            .chunks(2)
            .map(|c| c.join(" "))
            .chain(["t.sv".to_string()])
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();

    let read_outputs = |d: &std::path::Path| {
        (
            std::fs::read_to_string(d.join("h.txt")).ok(),
            std::fs::read_to_string(d.join("i.txt")).ok(),
            // …and nothing may appear beside the filelist instead.
            d.join("ip/h.txt").exists() || d.join("ip/i.txt").exists(),
        )
    };
    let clean = |d: &std::path::Path| {
        for p in ["h.txt", "i.txt", "ip/h.txt", "ip/i.txt"] {
            let _ = std::fs::remove_file(d.join(p));
        }
    };

    let mut argv: Vec<&str> = vec!["ip/t.sv"];
    argv.extend_from_slice(FLAGS);
    clean(&d);
    let cmdline = vita_in(&d, &argv, &[]);
    let cmdline_files = read_outputs(&d);

    clean(&d);
    let filelist = vita_in(&d, &["-F", "ip/all.f"], &[]);
    let filelist_files = read_outputs(&d);

    assert_eq!(cmdline, filelist, "argv vs -F frame diverged");
    assert_eq!(cmdline_files, filelist_files, "output files diverged");
    assert_eq!(cmdline.2, 0, "both must succeed:\n{}", cmdline.1);
    assert!(cmdline_files.0.is_some(), "--hier-tree wrote nothing");
    assert!(!cmdline_files.2, "an output landed beside the .f");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_output_path_from_a_filelist_lands_where_the_caller_stands() {
    // Same class, silent version: `--hier-tree h.txt` in a `-F` frame used to
    // resolve against the .f's directory, so the file appeared somewhere the
    // caller never named. Bucket-C/output paths anchor to the CWD, like `-l`.
    let d = tmpdir("outpath");
    std::fs::create_dir_all(d.join("ip")).unwrap();
    std::fs::write(
        d.join("ip/t.sv"),
        "module top; initial $display(\"ran\"); endmodule\n",
    )
    .unwrap();
    std::fs::write(d.join("ip/build.f"), "--hier-tree h.txt\nt.sv\n").unwrap();
    let (out, err, rc) = vita_in(&d, &["-F", "ip/build.f"], &[]);
    assert_eq!(rc, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(d.join("h.txt").exists(), "expected h.txt in the CWD");
    assert!(
        !d.join("ip/h.txt").exists(),
        "must not land in the .f's dir"
    );
    let _ = std::fs::remove_dir_all(&d);
}
