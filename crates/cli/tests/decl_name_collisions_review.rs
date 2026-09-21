//! Review round 1, fix round: the PACKAGE lane's routine pairs (M6), the per-SV-scope
//! duplicate test in a subroutine body and in a module block (F1), and the
//! declarator-drop that keeps a duplicate from cascading into "undeclared" (M8).
//!
//! Split out of `decl_name_collisions_kinds.rs` at the repo's 1000-line policy. Same
//! oracles: iverilog 13.0 (`-g2012`), Verilator 5.052 (`--binary --timing`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns `(exit_code, stdout+stderr)`.
fn run(src: &str) -> (i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dnc3_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (out.status.code().unwrap_or(-1), text)
}

/// The §3.13 refusal, pinned by code, name, both binders in declaration order, and
/// the note that points at the first declaration.
fn assert_collision(name: &str, src: &str, ident: &str, unit: &str, first: &str, dup: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, 1, "{name}: a refused design must exit 1:\n{out}");
    assert!(
        out.contains("VITA-E3009"),
        "{name}: the refusal is E3009:\n{out}"
    );
    // Two declarations of ONE kind get their own sentence — "both times as a port",
    // not "as a port and as a port".
    let want = if first == dup {
        format!("`{ident}` is declared twice in this {unit}, both times as {dup}")
    } else {
        format!("`{ident}` is declared twice in this {unit}: as {first} and as {dup}")
    };
    assert!(out.contains(&want), "{name}: expected `{want}`:\n{out}");
    assert!(
        out.contains("the first declaration of that name is here"),
        "{name}: a note points at the FIRST declaration:\n{out}"
    );
}

/// A design both oracles RUN. `needles` are the observed `$display` lines.
fn assert_runs(name: &str, src: &str, needles: &[&str]) {
    let (rc, out) = run(src);
    assert_eq!(rc, 0, "{name}: this design is legal and must run:\n{out}");
    for n in needles {
        assert!(out.contains(n), "{name}: expected `{n}`:\n{out}");
    }
}

// ─────────── review round 1: the package lane, and the two block lanes ──────────

#[test]
fn the_package_lane_refuses_a_routine_against_a_variable_or_a_parameter() {
    // M6. The package lane shipped routine-vs-routine only, so `package pk; int f;
    // function int f(…);` and the `parameter int f` twin stayed silent while the
    // MODULE twins of exactly those two pairs were refused (census p21_a / p21_c) —
    // an asymmetry, not a scope. iverilog "'f' … declared here as a variable" /
    // "… as a parameter"; verilator "Function has the same name as variable: 'f'" /
    // "… as local parameter: 'f'". vita printed `F16 44` / `F17 44`.
    assert_collision(
        "f16",
        "package pk;\n\
         \x20 int f;\n\
         \x20 function int f(input int a); return a + 1; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin #1 $display(\"O=%0d\", 44); $finish; end\n\
         endmodule\n",
        "f",
        "package",
        "a variable",
        "a function",
    );
    assert_collision(
        "f17",
        "package pk;\n\
         \x20 parameter int f = 7;\n\
         \x20 function int f(input int a); return a + 1; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin #1 $display(\"O=%0d\", 44); $finish; end\n\
         endmodule\n",
        "f",
        "package",
        "a parameter",
        "a function",
    );
    // Control: a package holding a variable, a parameter and a routine of THREE names
    // — `P 44` in all three tools. The non-routine pairs of a package body stay with
    // the binders that already own them (`package.rs`'s E-DUP-UNIT for
    // parameter-vs-variable, `param_dup.rs`'s §26.2 for parameter-vs-parameter), so
    // this lane must not start reporting them a second time.
    assert_runs(
        "pkg_ctl",
        "package pk;\n\
         \x20 int v;\n\
         \x20 parameter int p = 7;\n\
         \x20 function int f(input int a); return a + 1; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin #1 $display(\"P %0d\", f(43)); $finish; end\n\
         endmodule\n",
        &["P 44"],
    );
}

#[test]
fn a_nested_block_may_shadow_a_subroutine_body_local() {
    // F1 — THE control the first cut of item E did not have, and the false loud it
    // shipped. `hoisted_here` was keyed on the flattened net name, and
    // `block_local_scope_prefix` returns `None` for BOTH a task-body declarator and a
    // nested block's declarator, so two declarations in two DIFFERENT SV scopes shared
    // one key: `task tk; integer x; begin integer x; … end endtask` was refused
    // `net/variable top.$itask$tk$L.x redeclared` where PRE and BOTH oracles print
    // `inner x=3`. The set is keyed on the declaring block CHAIN now, which is the
    // question IEEE asks.
    for (tag, open, close, line) in [
        ("unnamed nested block", "begin", "end", "inner x=3"),
        ("named nested block", "begin : inn", "end", "inner x=3"),
        ("fork arm", "fork begin", "end join", "inner x=3"),
    ] {
        assert_runs(
            tag,
            &format!(
                "module top;\n\
                 \x20 task tk;\n\
                 \x20   integer x;\n\
                 \x20   {open}\n\
                 \x20     integer x;\n\
                 \x20     x = 3;\n\
                 \x20     $display(\"inner x=%0d\", x);\n\
                 \x20   {close}\n\
                 \x20 endtask\n\
                 \x20 initial tk();\n\
                 endmodule\n"
            ),
            &[line],
        );
    }
    // The E2 (module block) lane twin of the same shape, measured: `inner x=3` in
    // iverilog, verilator and vita.
    assert_runs(
        "module block nested shadow",
        "module top;\n\
         \x20 initial begin\n\
         \x20   integer x;\n\
         \x20   begin\n\
         \x20     integer x;\n\
         \x20     x = 3;\n\
         \x20     $display(\"inner x=%0d\", x);\n\
         \x20   end\n\
         \x20 end\n\
         endmodule\n",
        &["inner x=3"],
    );
    // …and the control that says the SAME-scope duplicate is still refused, so the
    // chain key did not simply turn the guard off.
    let (rc, out) = run("module top;\n\
         \x20 int r;\n\
         \x20 task tk(input int a);\n\
         \x20   begin\n\
         \x20     int x = 1;\n\
         \x20     int x = 3;\n\
         \x20     r = a + x;\n\
         \x20   end\n\
         \x20 endtask\n\
         \x20 initial begin tk(40); $display(\"x=%0d\", r); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(rc, 1, "two declarators in ONE nested block:\n{out}");
    assert!(
        out.contains("redeclared (duplicate declaration)"),
        "…still refused:\n{out}"
    );
}

#[test]
fn a_duplicate_declarator_does_not_cascade_into_undeclared_uses() {
    // M8. The E2 guard used to refuse the whole DECLARATION, so `int x, x;` lost the
    // clean declarator too and every later use reported `E3010 undeclared net/variable
    // top.x` — one page asserting that `top.x` is both redeclared and undeclared.
    // PRE printed one error and so do both oracles. The duplicate declarator is
    // dropped and the rest of the declaration goes on, so the name still binds.
    for (tag, decl) in [
        ("comma declaration", "int x, x;"),
        ("two declarations", "int x;\n\x20   int x;"),
    ] {
        let (rc, out) = run(&format!(
            "module top;\n\
             \x20 initial begin\n\
             \x20   {decl}\n\
             \x20   x = 3;\n\
             \x20   $display(\"x=%0d\", x);\n\
             \x20   $finish;\n\
             \x20 end\n\
             endmodule\n"
        ));
        assert_eq!(rc, 1, "{tag}: refused:\n{out}");
        assert_eq!(
            out.matches("error[VITA-").count(),
            1,
            "{tag}: exactly ONE error — no undeclared cascade:\n{out}"
        );
        assert!(
            out.contains("net/variable `top.x` redeclared (duplicate declaration)"),
            "{tag}: and it is the redeclare:\n{out}"
        );
        assert!(
            !out.contains("undeclared"),
            "{tag}: `top.x` must not also be called undeclared:\n{out}"
        );
    }
    // The MODULE-scope twin, byte-identical before and after this slice: one error.
    let (rc, out) = run("module top;\n\
         \x20 logic y, y;\n\
         \x20 initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(rc, 1, "module-scope twin:\n{out}");
    assert_eq!(
        out.matches("error[VITA-").count(),
        1,
        "module-scope twin is one error too:\n{out}"
    );
}

// ───────── review round 2: the carrier skip by IDENTITY, and its neighbours ─────────

#[test]
fn a_user_name_shaped_like_a_type_parameter_carrier_is_still_judged() {
    // S1 / S2 / S5. The first cut skipped every name containing a `$`; the second
    // narrowed that to the carrier SUFFIX grammar — and that was the same defect one
    // narrowing later, because `f$w`, `q$d0a` and `T$s` are legal simple identifiers
    // (IEEE §5.6). Two `function int f$w()` went from a W3056 warning to NO diagnostic
    // while still answering the SECOND body. The skip is an IDENTITY test now: a
    // `<stem>$…` carrier is exempt only where this unit's own parameter declarations
    // hold BOTH `<stem>$w` and `<stem>$s`, which is what the producer always and only
    // mints for a `parameter type`.
    //
    // iverilog "'f$w' has already been declared in this scope."; verilator "Duplicate
    // declaration of function: 'f$w'" / "… of function: 'q$d0a'" / "Function has the
    // same name as variable: 'T$s'".
    assert_collision(
        "s1_dollar_w_rtn",
        "module top;\n\
         \x20 function int f$w(); f$w = 44; endfunction\n\
         \x20 function int f$w(); f$w = 49; endfunction\n\
         \x20 initial begin $display(\"RD=%0d\", f$w()); #1 $finish; end\n\
         endmodule\n",
        "f$w",
        "module",
        "a function",
        "a function",
    );
    assert_collision(
        "s2b_dollar_d0a",
        "module top;\n\
         \x20 function int q$d0a(); q$d0a = 44; endfunction\n\
         \x20 function int q$d0a(); q$d0a = 49; endfunction\n\
         \x20 initial begin $display(\"RD=%0d\", q$d0a()); #1 $finish; end\n\
         endmodule\n",
        "q$d0a",
        "module",
        "a function",
        "a function",
    );
    assert_collision(
        "s2_dollar_s_net",
        "module top;\n\
         \x20 wire T$s;\n\
         \x20 function int T$s(); T$s = 44; endfunction\n\
         \x20 initial begin $display(\"O=%0d\", T$s()); #1 $finish; end\n\
         endmodule\n",
        "T$s",
        "module",
        "a net",
        "a function",
    );
    // THE control: two DIFFERENT carrier-shaped user names still run — `RD=44` in all
    // three tools.
    assert_runs(
        "s1_ctl",
        "module top;\n\
         \x20 function int f$w(); f$w = 44; endfunction\n\
         \x20 function int g$w(); g$w = 49; endfunction\n\
         \x20 initial begin $display(\"RD=%0d\", f$w()); #1 $finish; end\n\
         endmodule\n",
        &["RD=44"],
    );
}

#[test]
fn a_real_type_parameter_and_its_carriers_still_run() {
    // The other side of the identity test: a genuine `parameter type T` mints `T$w`,
    // `T$s` (and the dim carriers) as ParamDecls of this very unit, so the stem IS
    // registered and none of them may be reported. `D=3 32` in all three tools.
    assert_runs(
        "type parameter carriers",
        "module top #(parameter type T = int);\n\
         \x20 T v;\n\
         \x20 initial begin v = 3; $display(\"D=%0d %0d\", v, $bits(v)); #1 $finish; end\n\
         endmodule\n",
        &["D=3 32"],
    );
    // A user `logic T$w` BESIDE a real `parameter type T`: measured — both oracles RUN
    // it (`C=1 3 32`) and vita refuses it with a PRE-EXISTING constant-shadow error,
    // byte-identical before and after this slice. Pinned as the OBSERVED verdict so the
    // identity test cannot be blamed for it, and so the residue is visible: the §3.13
    // walk correctly says nothing here (the stem is registered), and the remaining
    // loud belongs to the constant-shadow gate.
    let (rc, out) = run("module top #(parameter type T = int);\n\
         \x20 logic T$w;\n\
         \x20 T v;\n\
         \x20 initial begin T$w = 1'b1; v = 3; $display(\"C=%0b %0d\", T$w, v); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(rc, 1, "pre-existing constant-shadow refusal:\n{out}");
    assert!(
        out.contains("resolves to a constant"),
        "…and it is the constant-shadow gate, not the §3.13 walk:\n{out}"
    );
    assert!(
        !out.contains("is declared twice in this"),
        "the §3.13 walk must say nothing about a registered carrier:\n{out}"
    );
}

#[test]
fn a_user_carrier_shaped_parameter_is_not_called_a_type_parameter() {
    // S5. `param_dup::display_name` split the name at its `$` and announced "type
    // parameter `Q`" for two user `localparam int Q$d0a` — a declaration and a keyword
    // the file does not contain. It asks the same identity test now.
    let (rc, out) = run("module top;\n\
         \x20 localparam int Q$d0a = 1;\n\
         \x20 localparam int Q$d0a = 2;\n\
         \x20 initial begin $display(\"Q=%0d\", Q$d0a); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(rc, 1, "refused:\n{out}");
    assert!(
        out.contains("duplicate declaration of parameter `Q$d0a`"),
        "the user's own name, and the `parameter` word:\n{out}"
    );
    assert!(
        !out.contains("type parameter"),
        "no type parameter is declared in this file:\n{out}"
    );
    // …and a REAL duplicated `parameter type T` is still reported by its stem, once.
    let (rc, out) = run(
        "module top #(parameter type T = int); parameter type T = byte;\n\
         \x20 T x;\n\
         \x20 initial begin x = 3; $display(\"b=%0d\", $bits(x)); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(rc, 1, "refused:\n{out}");
    assert!(
        out.contains("type parameter `T`") && !out.contains("T$w") && !out.contains("T$s"),
        "the stem, never the carrier:\n{out}"
    );
    assert_eq!(
        out.matches("duplicate declaration of").count(),
        1,
        "one user declaration, one report:\n{out}"
    );
}

#[test]
fn the_package_lane_sees_type_names_and_enum_labels_too() {
    // S3. The package guard sat ABOVE the `Typedef` arm, so a package's typedef, its
    // enum labels and its class names were never collected — and the module twin of
    // exactly that pair had just shipped. iverilog "'fn' has already been declared in
    // this scope … as an enum type or value"; verilator "Function has the same name as
    // ENUMITEM 'fn'".
    assert_collision(
        "s5_pkg_enum",
        "package pk;\n\
         \x20 typedef enum int { fn = 7 } e_t;\n\
         \x20 function int fn(); fn = 44; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin $display(\"O=%0d\", fn()); #1 $finish; end\n\
         endmodule\n",
        "fn",
        "package",
        "an enum label",
        "a function",
    );
    assert_collision(
        "package typedef",
        "package pk;\n\
         \x20 typedef int f;\n\
         \x20 function int f(); f = 44; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin $display(\"O=%0d\", 44); #1 $finish; end\n\
         endmodule\n",
        "f",
        "package",
        "a typedef",
        "a function",
    );
    // Control: distinct names run — `O=44` in all three.
    assert_runs(
        "s3_pkg_ctl",
        "package pk;\n\
         \x20 typedef enum int { aa = 7 } e_t;\n\
         \x20 function int fn(); fn = 44; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin $display(\"O=%0d\", fn()); #1 $finish; end\n\
         endmodule\n",
        &["O=44"],
    );
}

#[test]
fn a_nested_block_duplicate_names_the_block_it_lives_in() {
    // S4. The refusal was reported from the OUTER prefix, so it printed
    // `top.$itask$tk$L.x` for a declarator whose net is `top.$itask$tk$L.$blk$<lo>.x` —
    // a key no lookup can reach. It is reported from inside the declaring block's own
    // scope now, so the key and the `[in …]` frame both name where the net lives.
    let (rc, out) = run("module top;\n\
         \x20 task tk;\n\
         \x20   begin\n\
         \x20     integer x;\n\
         \x20     integer x;\n\
         \x20     x = 3;\n\
         \x20     $display(\"x=%0d\", x);\n\
         \x20   end\n\
         \x20 endtask\n\
         \x20 initial tk();\n\
         endmodule\n");
    assert_eq!(rc, 1, "still refused (both oracles reject):\n{out}");
    assert!(
        out.contains("$itask$tk$L.$blk$") && out.contains("redeclared (duplicate declaration)"),
        "the key carries the declaring block's segment:\n{out}"
    );
    assert!(
        !out.contains("`top.$itask$tk$L.x` redeclared"),
        "…and not the outer-prefix key, which names no net:\n{out}"
    );
}

// ───────────── review round 3: the two sentences, measured per spelling ─────────────

#[test]
fn a_generate_block_label_only_gets_the_transparency_clause_inside_a_region() {
    // T1. A generate block's LABEL is declared in the ENCLOSING scope by §27.6 whether
    // or not `generate … endgenerate` wraps it, and `collect_gen_items` stamped every
    // label `in_region = true` — so `if (1) begin : fn … end` (no `generate` keyword
    // anywhere in the file) was told its collision came through "a `generate …
    // endgenerate` region", a construct the file does not contain. Both spellings are
    // refused either way: iverilog "'fn' has already been declared in this scope.",
    // verilator "Generate block has the same name as function: 'fn'".
    let bare = "module top;\n\
         \x20 function int fn(); fn = 44; endfunction\n\
         \x20 initial begin $display(\"O=%0d\", fn()); #1 $finish; end\n\
         \x20 if (1) begin : fn\n\
         \x20   wire z;\n\
         \x20 end\n\
         endmodule\n";
    assert_collision(
        "g2_nogen_label",
        bare,
        "fn",
        "module",
        "a function",
        "a generate block",
    );
    let (_rc, out) = run(bare);
    assert!(
        !out.contains("endgenerate"),
        "no `generate … endgenerate` is in this file, so none may be in the message:\n{out}"
    );
    // The same block INSIDE an explicit region keeps the §27.2/§27.3 clause, which is
    // true there: the region really is transparent and really does put the label in
    // the module's scope.
    let wrapped = "module top;\n\
         \x20 function int fn(); fn = 44; endfunction\n\
         \x20 initial begin $display(\"O=%0d\", fn()); #1 $finish; end\n\
         \x20 generate if (1) begin : fn\n\
         \x20   wire z;\n\
         \x20 end endgenerate\n\
         endmodule\n";
    assert_collision(
        "g1_trailing_gen",
        wrapped,
        "fn",
        "module",
        "a function",
        "a generate block",
    );
    let (_rc, out) = run(wrapped);
    assert!(
        out.contains("region with no block label is TRANSPARENT"),
        "inside a region the clause is true and must be there:\n{out}"
    );
}

#[test]
fn a_same_kind_pair_does_not_stutter() {
    // R3-2. "as a port and as a port" reads as a typo; one kind gets one clause.
    for (tag, src, ident, both) in [
        (
            "two functions",
            "module top;\n\
             \x20 function int f(); f = 44; endfunction\n\
             \x20 function int f(); f = 49; endfunction\n\
             \x20 initial begin $display(\"RD=%0d\", f()); #1 $finish; end\n\
             endmodule\n",
            "f",
            "a function",
        ),
        (
            "two ports",
            "module top(nn);\n\
             \x20 input nn;\n\
             \x20 input nn;\n\
             \x20 initial begin #1 $finish; end\n\
             endmodule\n",
            "nn",
            "a port",
        ),
        (
            "two genvars",
            "module top;\n\
             \x20 genvar Q;\n\
             \x20 genvar Q;\n\
             \x20 initial begin #1 $finish; end\n\
             endmodule\n",
            "Q",
            "a genvar",
        ),
    ] {
        let (rc, out) = run(src);
        assert_eq!(rc, 1, "{tag}: refused:\n{out}");
        assert!(
            out.contains(&format!(
                "`{ident}` is declared twice in this module, both times as {both}"
            )),
            "{tag}: the same-kind sentence:\n{out}"
        );
        assert!(
            !out.contains(&format!("as {both} and as {both}")),
            "{tag}: and never the stuttering one:\n{out}"
        );
    }
    // The two-kind sentence is untouched.
    let (_rc, out) = run("module top;\n\
         \x20 wire f;\n\
         \x20 function int f(); f = 44; endfunction\n\
         \x20 initial begin #1 $finish; end\n\
         endmodule\n");
    assert!(
        out.contains("`f` is declared twice in this module: as a net and as a function"),
        "{out}"
    );
}
