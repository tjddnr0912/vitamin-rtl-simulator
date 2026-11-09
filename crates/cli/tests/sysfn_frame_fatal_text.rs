//! The F4004 text `fatal_frame_sysread` (sim-engine `state/frame_eval.rs`) prints for each
//! system function a class-method body cannot perform, pinned verbatim (ROADMAP §5.2
//! SYS-1a, B6): ten of its arms read the one id -> name home (sim-ir `names.rs`
//! `sysfunc_name`), and three keep the diagnostic's own phrase (a seeded `$random`, a
//! seeded `$dist_*`, a queue pop). Until this file no committed pin read the text: the
//! frame-executor pins assert only the code `F4004` (`r22_stmt_effect_executor.rs`).
//!
//! Each cell is the class-method shape of `r22_stmt_effect_executor.rs`
//! `every_statement_effect_is_loud_where_it_cannot_be_routed`; every expected line was
//! measured on the PRE binary (4a8f086b, the SYS-1a premise judge's K8 cells).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Combined stdout+stderr and whether the exit was 0, in a temp dir of its own.
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_sysfn_fatal_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("t.sv");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .arg("--timeout")
        .arg("400")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
    (s, out.status.success())
}

const TAIL: &str = " does its work as a statement-level effect, which the synchronous `&self` \
frame executor cannot perform — the call would return 0 and leave its destination untouched, so \
the run stops here rather than continuing on that value. This is one of the few positions vita \
cannot route to the statement executor: a class-method body, a CONTINUOUSLY re-evaluated \
expression (`assign`, `force`, a `wait` condition), or an intra-assignment delay \
(`x = #1 f(...)`). It DOES work in a module process, and in a task or function called from a \
statement — with or without `automatic`, with or without output formals. Call it there, assign \
the result to a variable, and use that variable here. [at time 0]";

#[test]
fn frame_fatal_names_each_system_function() {
    // (the text between the backticks, the method's declarations, its body)
    let cells: [(&str, &str, &str); 12] = [
        (
            "$fgets",
            "int fd; string s; int rc;",
            "fd = 0; rc = $fgets(s, fd); return rc;",
        ),
        (
            "$fread",
            "int fd; int n; logic [31:0] v;",
            "fd = 0; n = $fread(v, fd); return n;",
        ),
        (
            "$fscanf",
            "int fd; int n; int a;",
            r#"fd = 0; n = $fscanf(fd, "%d", a); return n;"#,
        ),
        (
            "$sscanf",
            "int n; int a;",
            r#"n = $sscanf("12", "%d", a); return n;"#,
        ),
        (
            "$fgetc",
            "int fd; int c;",
            "fd = 0; c = $fgetc(fd); return c;",
        ),
        (
            "$feof",
            "int fd; int e;",
            "fd = 0; e = $feof(fd); return e;",
        ),
        (
            "$ungetc",
            "int fd; int u;",
            "fd = 0; u = $ungetc(65, fd); return u;",
        ),
        (
            "$fopen",
            "int nfd;",
            r#"nfd = $fopen("o.txt", "w"); return nfd;"#,
        ),
        (
            "$value$plusargs",
            "int lim = 20; int rc;",
            r#"rc = $value$plusargs("LIM=%d", lim); return lim;"#,
        ),
        ("$cast", "int d = 0; int ok;", "ok = $cast(d, 7); return d;"),
        (
            "a seeded `$random`",
            "int seed = 1; int a;",
            "a = $random(seed); return seed;",
        ),
        (
            "a seeded `$dist_*`",
            "int seed = 1; int a;",
            "a = $dist_uniform(seed, 0, 99); return seed;",
        ),
    ];
    for (which, decls, body) in cells {
        let (o, ok) = run(&format!(
            r#"`timescale 1ns/1ps
class C;
  function int act ();
    {decls}
    {body}
  endfunction
endclass
module t;
  initial begin C c; int r; c = new(); r = c.act(); $display("r=%0d", r); $finish; end
endmodule"#
        ));
        assert!(!ok, "{which}: expected a failing exit:\n{o}");
        let want = format!("fatal[VITA-F4004] F-RUN-FATAL: `{which}`{TAIL}");
        assert!(
            o.lines().any(|l| l == want),
            "{which}: the F4004 line, verbatim:\n{want}\n--- got:\n{o}"
        );
        assert!(
            o.lines().any(|l| l == "simulation ended (Error) at time 0"),
            "{which}: the run must end as Error:\n{o}"
        );
        assert!(
            !o.contains("r="),
            "{which}: nothing printed after the fatal:\n{o}"
        );
    }
}
