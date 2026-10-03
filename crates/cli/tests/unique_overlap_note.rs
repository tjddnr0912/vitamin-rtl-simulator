//! `I-PARSE-UNIQUE-OVERLAP-UNCHECKED` (VITA-I2021) — vita does not check a `unique` /
//! `unique0` case or if for more than one match, and one Info line says so.
//!
//! IEEE 1800-2017 §12.4.2 (if) and §12.5.3 (case): a `unique` or `unique0` statement is
//! violated when more than one condition or case item matches; the implementation
//! issues a violation report and runs the FIRST match. `priority` has no such rule.
//! vita runs the first match (a first-match-wins cascade) and reports nothing: a §8
//! non-goal (manual 006 §1.4). This file pins that the log says so: once per parse, at
//! the first written `unique` / `unique0` qualifier, reached or not, as Info (counted
//! under `notes=`, `-Wno-`-able, never promoted, exit unchanged).
//!
//! Oracles, quoted verbatim above each pin: iverilog 13.0 (`iverilog -g2012`, `vvp -n`)
//! and verilator 5.052 (`--binary --timing --assert`). iverilog prints a compile-time
//! `sorry` per ELABORATED case site and rejects `unique if` at parse; verilator checks
//! the overlap at run time. Neither is the model for the count: one line per parse is
//! the row's decision (once per site would repeat a fact that holds for every site).
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const MSG: &str = "`unique` / `unique0` overlaps are not checked: when more than one case \
                   item or `if` condition matches, the first one runs and no violation is \
                   reported, where IEEE 1800-2017 §12.4.2 and §12.5.3 require one. Printed \
                   once, at the first such statement in source order";

struct Out {
    out: String,
    err: String,
    code: i32,
}

fn scratch() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_s586_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita_in(dir: &Path, args: &[&str]) -> Out {
    let o = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita");
    Out {
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
        code: o.status.code().unwrap_or(-1),
    }
}

/// Write `files` into a fresh directory and run `vita <args>` there.
fn run_files(files: &[(&str, &str)], args: &[&str]) -> Out {
    let d = scratch();
    for (name, text) in files {
        std::fs::write(d.join(name), text).unwrap();
    }
    let o = vita_in(&d, args);
    let _ = std::fs::remove_dir_all(&d);
    o
}

/// `vita <args> t.sv` on one design.
fn run(src: &str, args: &[&str]) -> Out {
    let mut a = args.to_vec();
    a.push("t.sv");
    run_files(&[("t.sv", src)], &a)
}

/// The location of every I2021 line (`t.sv:6:5`), in print order.
fn notes(err: &str) -> Vec<String> {
    err.lines()
        .filter(|l| l.contains("[VITA-I2021]"))
        .map(|l| l.split(": ").next().unwrap_or("").to_string())
        .collect()
}

/// `$display` output followed by the end-of-run line.
fn ended(lines: &str, t: u32) -> String {
    format!("{lines}simulation ended (Finish) at time {t}\n")
}

const C01_UCASEZ: &str = "module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b11;
    #1;
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";

#[test]
fn an_overlapping_unique_casez_runs_the_first_item_and_says_the_overlap_is_not_checked() {
    // iverilog: `t.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`
    //           then `y=1`.
    // verilator: `[1] %Error: t.sv:6: Assertion failed in t: unique case, but multiple
    //            matches found for '2'h3'` then `y=1`.
    let o = run(C01_UCASEZ, &[]);
    let line = format!("t.sv:6:5: info[VITA-I2021] I-PARSE-UNIQUE-OVERLAP-UNCHECKED: {MSG}");
    assert_eq!(
        o.err
            .lines()
            .filter(|l| l.contains("I2021"))
            .collect::<Vec<_>>(),
        vec![line.as_str()],
        "{}",
        o.err
    );
    assert_eq!(o.out, ended("y=1\n", 2));
    assert_eq!(
        o.code, 0,
        "an Info line never changes the exit code:\n{}",
        o.err
    );
    assert_eq!(
        o.err.lines().last(),
        Some("errors=0 warnings=1 notes=1"),
        "counted under notes=, W1017 is the one warning:\n{}",
        o.err
    );
}

#[test]
fn unique0_and_unique_if_are_announced_too() {
    // c04 `unique0 case`, items 01/01, r=01:
    //   iverilog: `t.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`
    //   verilator: `[1] %Error: t.sv:6: Assertion failed in t: unique0 case, but multiple
    //              matches found for '2'h1'`; both `y=1`.
    // c05 `unique if (a) … else if (b)` and c06 `unique0 if …`, a=b=1:
    //   iverilog: `t.sv:6: syntax error` (it has no `unique if`).
    //   verilator: `[1] %Error: t.sv:6: Assertion failed in t: 'unique if' statement
    //              violated` for both, and `y=0` — neither branch, its own quirk; IEEE
    //              runs the first true condition (vita `y=1`).
    let u0case = "module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b01;
    #1;
    unique0 case (r)
      2'b01: y = 1;
      2'b01: y = 2;
    endcase
    $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let uif = |q: &str| {
        format!(
            "module t;
  logic a, b; int y;
  initial begin
    a = 1; b = 1;
    #1;
    {q} if (a) y = 1;
    else if (b) y = 2;
    $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"
        )
    };
    for (what, src) in [
        ("unique0 case", u0case.to_string()),
        ("unique if", uif("unique")),
        ("unique0 if", uif("unique0")),
    ] {
        let o = run(&src, &[]);
        assert_eq!(notes(&o.err), ["t.sv:6:5"], "{what}:\n{}", o.err);
        assert_eq!(o.out, ended("y=1\n", 2), "{what}");
        assert_eq!(o.code, 0, "{what}");
    }
}

#[test]
fn priority_alone_is_never_announced() {
    // IEEE defines no multiple-match rule for `priority`, and both oracles are silent:
    // c07 `priority casez` overlap: iverilog `y=1`, no sorry; verilator `y=1`.
    // c08 `priority if`, a=b=1: iverilog `t.sv:6: syntax error`; verilator `y=1`.
    // c22 `priority case` + default: iverilog `y=1`, no sorry; verilator `y=1`.
    let pcase = C01_UCASEZ.replace("unique casez", "priority casez");
    let pif = "module t;
  logic a, b; int y;
  initial begin
    a = 1; b = 1;
    #1;
    priority if (a) y = 1;
    else if (b) y = 2;
    $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let pdefault = "module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b01;
    #1;
    priority case (r)
      2'b00: y = 0;
      2'b01: y = 1;
      default: y = 3;
    endcase
    $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    for src in [pcase.as_str(), pif, pdefault] {
        let o = run(src, &[]);
        assert!(!o.err.contains("[VITA-I2021]"), "{src}\n{}", o.err);
        assert!(o.err.contains("notes=0"), "{}", o.err);
        assert_eq!(o.out, ended("y=1\n", 2));
    }
}

#[test]
fn the_line_names_the_first_unique_qualifier_not_the_first_qualifier() {
    // A `priority casez` at line 5 comes first; the line goes to the `unique0` at 9.
    let src = "module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b11;
    priority casez (r)
      2'b?1: y = 1;
      default: y = 0;
    endcase
    unique0 case (r)
      2'b11: y = 3;
    endcase
    #1 $finish;
  end
endmodule
";
    let o = run(src, &[]);
    assert_eq!(notes(&o.err), ["t.sv:9:5"], "{}", o.err);
}

#[test]
fn two_sites_in_one_parse_print_one_line_at_the_first() {
    // c16 `unique casez` (line 6) + `unique0 casez` (line 10), both overlapping:
    //   iverilog: `t.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`
    //             `t.sv:10: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`
    //   verilator: `[1] %Error: t.sv:6: Assertion failed in t: unique case, but multiple
    //              matches found for '2'h3'` and `[1] %Error: t.sv:10: Assertion failed
    //              in t: unique0 case, but multiple matches found for '2'h3'`.
    // c23 two `unique case` with a default, no overlap (lines 6, 11):
    //   iverilog: the same sorry at `t.sv:6` and `t.sv:11`; verilator silent.
    let two_overlapping = "module t;
  logic [1:0] r; int y, z;
  initial begin
    r = 2'b11;
    #1;
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    unique0 casez (r)
      2'b?1: z = 1;
      2'b1?: z = 2;
    endcase
    $display(\"y=%0d z=%0d\", y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let two_disjoint = "module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b01;
    #1;
    unique case (r)
      2'b00: y = 0;
      2'b01: y = 1;
      default: y = 3;
    endcase
    unique case (r)
      2'b00: y = 0;
      2'b01: y = 1;
      default: y = 3;
    endcase
    $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    for (src, out) in [(two_overlapping, "y=1 z=1\n"), (two_disjoint, "y=1\n")] {
        let o = run(src, &[]);
        assert_eq!(notes(&o.err), ["t.sv:6:5"], "{src}\n{}", o.err);
        assert!(o.err.contains("notes=1"), "{}", o.err);
        assert_eq!(o.out, ended(out, 2));
    }
}

#[test]
fn the_first_site_follows_the_command_line_file_order() {
    let a = "module a(input logic [1:0] r, output int y);
  always_comb unique case (r) 2'd0: y = 0; default: y = 1; endcase
endmodule
";
    let b = "module t;
  logic [1:0] r; int y;
  a u(.r(r), .y(y));
  initial begin
    r = 0;
    unique0 if (r == 0) $display(\"b\"); else if (r == 1) $display(\"c\");
    #1 $finish;
  end
endmodule
";
    let files = [("a.sv", a), ("b.sv", b)];
    let ab = run_files(&files, &["a.sv", "b.sv"]);
    assert_eq!(notes(&ab.err), ["a.sv:2:15"], "{}", ab.err);
    let ba = run_files(&files, &["b.sv", "a.sv"]);
    assert_eq!(notes(&ba.err), ["b.sv:6:5"], "{}", ba.err);
    assert_eq!(ab.out, ba.out);
    assert_eq!(ab.out, ended("b\n", 1));
}

#[test]
fn the_location_resolves_through_include_macro_and_ifdef() {
    // `include`: iverilog `./inc.svh:4: vvp.tgt sorry: Case unique/unique0 qualities are
    // ignored.` then `t.sv:7: …` (the same text); verilator `[0] %Error: inc.svh:4:
    // Assertion failed in t.chk: unique0 case, but multiple matches found for '2'h3'`.
    let inc = "// header
task automatic chk(input logic [1:0] r, output logic y);
  y = 0;
  unique0 casez (r)
    2'b?1: y = 1;
    2'b1?: y = 1;
  endcase
endtask
";
    let top = "module t;
  logic [1:0] r; logic y, z;
  `include \"inc.svh\"
  initial begin
    r = 2'b11;
    chk(r, y);
    unique case (r)
      2'b11: z = 1;
      default: z = 0;
    endcase
    $display(\"y=%0d z=%0d\", y, z);
    $finish;
  end
endmodule
";
    let o = run_files(&[("inc.svh", inc), ("t.sv", top)], &["t.sv"]);
    assert_eq!(notes(&o.err), ["inc.svh:4:3"], "{}", o.err);
    assert_eq!(o.out, ended("y=1 z=1\n", 0));

    // A macro that expands to the qualifier: the call site.
    let mac = "`define UCASE(sel) unique case (sel)
module t;
  logic [1:0] r; int y;
  initial begin
    r = 0;
    `UCASE(r) 2'd0: y = 0; default: y = 1; endcase
    #1 $finish;
  end
endmodule
";
    let o = run(mac, &[]);
    assert_eq!(notes(&o.err), ["t.sv:6:5"], "{}", o.err);

    // A qualifier the preprocessor drops is not written; with the define it is.
    let ifdef = "module t;
  logic [1:0] r; int y;
  initial begin
`ifdef NOPE
    unique case (r) 2'd0: y = 0; endcase
`endif
    case (r) 2'd0: y = 0; endcase
    #1 $finish;
  end
endmodule
";
    let o = run(ifdef, &[]);
    assert_eq!(notes(&o.err), Vec::<String>::new(), "{}", o.err);
    assert!(o.err.contains("notes=0"), "{}", o.err);
    let o = run(ifdef, &["-DNOPE"]);
    assert_eq!(notes(&o.err), ["t.sv:5:5"], "{}", o.err);
}

#[test]
fn a_qualifier_on_its_own_line_is_located_at_the_qualifier() {
    // `unique` / `casez` / `(r)` on lines 6 / 7 / 8:
    //   iverilog: `t.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`
    //   verilator: `[1] %Error: t.sv:7: Assertion failed in t: unique case, but multiple
    //              matches found for '2'h3'` (the `casez` line).
    let src = C01_UCASEZ.replace(
        "    unique casez (r)\n",
        "    unique\n      casez\n        (r)\n",
    );
    let o = run(&src, &[]);
    assert_eq!(notes(&o.err), ["t.sv:6:5"], "{}", o.err);
    assert_eq!(o.out, ended("y=1\n", 2));
}

#[test]
fn an_unreached_site_is_still_announced() {
    // The line claims nothing about reachability: the cut holds for every written site.
    // iverilog differs per case, recorded here:
    //   uninstantiated module, `-s t`: no sorry (vita `--top t`).
    //   untaken generate branch: no sorry.
    //   uncalled function: `t.sv:4: vvp.tgt sorry: Case unique/unique0 qualities are
    //   ignored.`
    // verilator: silent on all three (nothing runs). All three print `y=5`.
    let uninst = "module unused(input logic [1:0] r, output int y);
  always_comb begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
endmodule
module t;
  int y;
  initial begin
    y = 5;
    #1 $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let untaken = "module t #(parameter P = 0);
  logic [1:0] r; int y;
  generate if (P == 1) begin : g
    always @(r) begin
      unique casez (r)
        2'b?1: y = 1;
        2'b1?: y = 2;
      endcase
    end
  end endgenerate
  initial begin
    y = 5;
    r = 2'b11;
    #1 $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let uncalled = "module t;
  function automatic int f(input logic [1:0] r);
    int y;
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    return y;
  endfunction
  int y;
  initial begin
    y = 5;
    #1 $display(\"y=%0d\", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    for (src, args, at) in [
        (uninst, &["--top", "t"][..], "t.sv:3:5"),
        (untaken, &[][..], "t.sv:5:7"),
        (uncalled, &[][..], "t.sv:4:5"),
    ] {
        let o = run(src, args);
        assert_eq!(notes(&o.err), [at], "{src}\n{}", o.err);
        assert_eq!(o.out, ended("y=5\n", 2), "{src}");
        assert_eq!(o.code, 0);
    }
}

#[test]
fn a_design_that_does_not_parse_prints_only_its_syntax_error() {
    // Byte-for-byte the stderr from before this code existed: the line is printed only
    // after every parse-stage return.
    let src = "module t;
  logic [1:0] r; int y;
  initial begin
    unique case (r) 2'd0: y = 0; endcase
    y = ;
  end
endmodule
";
    let o = run(src, &[]);
    assert_eq!(
        o.err,
        "t.sv:5:9: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found ';'\n\
         errors=1 warnings=0 notes=0\n"
    );
    assert_eq!(o.code, 1);
}

#[test]
fn a_source_with_no_design_unit_prints_only_its_error_on_vita_and_vcmp() {
    // A compilation-unit function holding the qualifier, and nothing else: it parses, but
    // there is no design unit, which is the parse stage's last refusal. Both commands
    // print exactly what they printed before this code existed.
    let src = "function automatic int f(int x);
  unique case (x) 0: return 1; default: return 2; endcase
endfunction
";
    let want = "error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: no design units found in source\n\
                errors=1 warnings=0 notes=0\n";
    let o = run(src, &[]);
    assert_eq!(o.err, want, "one-shot");
    assert_eq!(o.code, 1);
    let c = run(src, &["vcmp", "-o", "t.vu"]);
    assert_eq!(c.err, want, "vcmp");
    assert_eq!(c.code, 1);
}

#[test]
fn a_select_base_warning_before_the_first_unique_does_not_take_its_place() {
    // W2004 (line 4) is recorded before the first qualifier (line 5). The once-latch
    // must key on its own kind: any earlier parse warning is not "already said".
    let src = "module t;
  logic [15:0] a = 16'h1234; logic [7:0] o; logic [1:0] r; int y;
  initial begin
    o = (a >> 8)[7:0];
    unique case (r) 2'd0: y = 0; default: y = 1; endcase
    $display(\"o=%02h y=%0d\", o, y);
    #10 $finish;
  end
endmodule
";
    let o = run(src, &[]);
    let codes: Vec<&str> = o
        .err
        .lines()
        .filter_map(|l| l.split("[VITA-").nth(1)?.split(']').next())
        .collect();
    assert_eq!(codes, ["W2004", "I2021", "W1017"], "{}", o.err);
    assert_eq!(notes(&o.err), ["t.sv:5:5"], "{}", o.err);
    assert_eq!(o.out, ended("o=12 y=1\n", 10));
}

#[test]
fn the_line_comes_after_the_parse_warnings_and_before_the_timescale_warning() {
    // `unique` at line 4, a W2004 select-base at line 5: W2004 is a parse-time warning
    // printed before the parse-error gate; I2021 is printed after it.
    let src = "module t;
  logic [15:0] a = 16'h1234; logic [7:0] o; logic [1:0] r; int y;
  initial begin
    unique case (r) 2'd0: y = 0; default: y = 1; endcase
    o = (a >> 8)[7:0];
    $display(\"o=%02h\", o);
    $finish;
  end
endmodule
";
    let o = run(src, &[]);
    let codes: Vec<&str> = o
        .err
        .lines()
        .filter_map(|l| l.split("[VITA-").nth(1)?.split(']').next())
        .collect();
    assert_eq!(codes, ["W2004", "I2021", "W1017"], "{}", o.err);
    assert_eq!(notes(&o.err), ["t.sv:4:5"]);
    assert_eq!(o.out, ended("o=12\n", 0));
}

#[test]
fn the_line_is_info_suppressible_never_promoted() {
    // A `timescale, so W1017 is absent and `-Werror` has no warning to promote.
    let src = "`timescale 1ns/1ns
module t;
  logic [1:0] r; int y;
  initial begin
    r = 0;
    unique case (r) 2'd0: y = 0; default: y = 1; endcase
    #1 $finish;
  end
endmodule
";
    let plain = run(src, &[]);
    assert_eq!(notes(&plain.err), ["t.sv:6:5"], "{}", plain.err);
    assert!(plain.err.contains(": info[VITA-I2021] "), "{}", plain.err);

    for flag in ["-Werror", "-Werror=I2021"] {
        let o = run(src, &[flag]);
        assert_eq!(o.code, 0, "`{flag}` does not promote Info:\n{}", o.err);
        assert_eq!(notes(&o.err), ["t.sv:6:5"], "{flag}\n{}", o.err);
        assert!(o.err.contains(": info[VITA-I2021] "), "{flag}\n{}", o.err);
        assert_eq!(
            o.err.lines().last(),
            Some("errors=0 warnings=0 notes=1"),
            "{flag}"
        );
    }
    for flag in [
        "-Wno-I2021",
        "-Wno-VITA-I2021",
        "-Wno-I-PARSE-UNIQUE-OVERLAP-UNCHECKED",
    ] {
        let o = run(src, &[flag]);
        assert_eq!(notes(&o.err), Vec::<String>::new(), "{flag}\n{}", o.err);
        assert_eq!(
            o.err.lines().last(),
            Some("errors=0 warnings=0 notes=0"),
            "{flag}"
        );
        assert_eq!(o.out, plain.out, "{flag}");
    }
    let q = run(src, &["-q"]);
    assert_eq!(
        notes(&q.err),
        ["t.sv:6:5"],
        "`-q` quiets stdout only:\n{}",
        q.err
    );
}

#[test]
fn every_backend_prints_it_and_the_staged_flow_prints_it_at_vcmp_only() {
    let src = C01_UCASEZ;
    let mut oneshot: Vec<Out> = Vec::new();
    for be in ["native", "interp", "vm"] {
        oneshot.push(run(src, &["--backend", be]));
    }
    for (be, o) in ["interp", "vm"].iter().zip(&oneshot[1..]) {
        assert_eq!(o.out, oneshot[0].out, "{be} stdout");
        assert_eq!(o.err, oneshot[0].err, "{be} stderr");
    }
    assert_eq!(notes(&oneshot[0].err), ["t.sv:6:5"]);

    // The line is printed by the parse; `velab` and `vrun` read artifacts, never parse.
    let d = scratch();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let c = vita_in(&d, &["vcmp", "-o", "t.vu", "t.sv"]);
    assert_eq!(c.code, 0, "{}", c.err);
    assert_eq!(notes(&c.err), ["t.sv:6:5"], "vcmp:\n{}", c.err);
    let e = vita_in(&d, &["velab", "-o", "t.velab", "t.vu"]);
    assert_eq!(e.code, 0, "{}", e.err);
    assert_eq!(notes(&e.err), Vec::<String>::new(), "velab:\n{}", e.err);
    let r = vita_in(&d, &["vrun", "t.velab"]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(notes(&r.err), Vec::<String>::new(), "vrun:\n{}", r.err);
    assert_eq!(r.out, oneshot[0].out, "staged stdout == one-shot stdout");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_unique_array_method_is_not_a_qualifier() {
    // `q.unique()` (IEEE §7.12.1 locator) uses the same keyword in member position.
    let src = "module t;
  int q[$] = '{1, 1, 2};
  int r[$];
  initial begin
    r = q.unique();
    $display(\"n=%0d\", r.size());
    $finish;
  end
endmodule
";
    let o = run(src, &[]);
    assert!(!o.err.contains("[VITA-I2021]"), "{}", o.err);
    assert!(o.err.contains("notes=0"), "{}", o.err);
    assert_eq!(o.out, ended("n=2\n", 0));
}
