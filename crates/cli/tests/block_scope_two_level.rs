//! R16 §3.4: the same name reused at TWO nesting levels of sibling block trees.
//!
//! v1 gives an `automatic` (or dynamic-storage) block-local its own `$blk$<lo>` net when
//! the name is declared in ≥2 mutually disjoint blocks. That worked at one level and not
//! at two: with an outer name AND an inner name both shared, the inner locals lost their
//! scoping, flattened onto one net, and collided.
//!
//! The cause was a disagreement between the two phases. The Logic phase lowers a scoped
//! block's body inside `with_scope("$blk$<lo>")`, so with both levels scoped the segments
//! NEST; the Nets-phase hoist recursed FLAT. The inner block's nets were created at
//! `t.$blk$<inner>` while its body resolved under `t.$blk$<outer>.$blk$<inner>`, missed
//! them, and fell through to the module. The classifier avoided that by dropping every
//! candidate block nested inside another candidate — which is exactly why one level
//! worked and two did not.
//!
//! The hoist now nests the same way the lowering does, so the drop is gone. A second
//! adjustment came with it, and was found by the never-emitted guard rather than by a
//! test: block-local initializers recorded under `t.$blk$<outer>.$blk$<inner>` were
//! claimed by no flush point, because the claim rule only accepted a DIRECT `$blk$`
//! child. It now accepts any depth of `$blk$` segments, which is what "in this scope"
//! means once blocks can nest their scopes.
//!
//! The shape matters because it is the standard table-driven walker:
//! `foreach (files[fi]) begin automatic int fd = $fopen(…); … begin <inner locals> end
//! end`, repeated for several files.
//!
//! ORACLE. iverilog rejects an explicit `automatic` lifetime override, so each case is
//! pinned against the same program with the locals un-keyworded inside an `automatic`
//! task, where IEEE 1800 §6.21 makes them automatic. Those runs are quoted per test.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bs2l_{}_{n}", std::process::id()));
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
    (text, out.status.success())
}

fn runs(src: &str, want: &[&str]) {
    let (o, ok) = run(src);
    assert!(ok, "expected acceptance, got:\n{o}");
    let got: Vec<&str> = o.lines().filter(|l| l.starts_with("R ")).collect();
    assert_eq!(got, want, "output mismatch:\n{o}");
}

/// The report's reproducer: `fd` shared at the outer level, `n`/`n_skip`/`msg` shared at
/// the inner one. Four diagnostics at 6b6b8ef. iverilog prints `R A 1 1 1` / `R B 2 1 2`.
#[test]
fn same_name_at_two_levels() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int fd = 1;
                 begin
                   automatic int  n = 0, n_skip = 0;
                   automatic byte msg [];
                   msg = new[fd]; n += msg.size(); n_skip++;
                   $display("R A %0d %0d %0d", n, n_skip, msg.size());
                 end
               end
               begin
                 automatic int fd = 2;
                 begin
                   automatic int  n = 0, n_skip = 0;
                   automatic byte msg [];
                   msg = new[fd]; n += msg.size(); n_skip++;
                   $display("R B %0d %0d %0d", n, n_skip, msg.size());
                 end
               end
             end
           endmodule"#,
        &["R A 1 1 1", "R B 2 1 2"],
    );
}

/// The report's PASS boundary: unique OUTER names, shared inner ones — already worked.
#[test]
fn unique_outer_names_still_work() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int fd_a = 1;
                 begin automatic int n = 0; $display("R A %0d %0d", fd_a, n); end
               end
               begin
                 automatic int fd_b = 2;
                 begin automatic int n = 0; $display("R B %0d %0d", fd_b, n); end
               end
             end
           endmodule"#,
        &["R A 1 0", "R B 2 0"],
    );
}

/// The other PASS boundary: shared outer names, unique inner ones.
#[test]
fn unique_inner_names_still_work() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int fd = 1;
                 begin automatic int na = 0; $display("R A %0d %0d", fd, na); end
               end
               begin
                 automatic int fd = 2;
                 begin automatic int nb = 0; $display("R B %0d %0d", fd, nb); end
               end
             end
           endmodule"#,
        &["R A 1 0", "R B 2 0"],
    );
}

/// THREE levels — the nesting rule is not special-cased to two. iverilog prints
/// `R A 1 10 100` / `R B 2 20 200`.
#[test]
fn same_name_at_three_levels() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int a = 1;
                 begin
                   automatic int b = 10;
                   begin automatic int c = 100; $display("R A %0d %0d %0d", a, b, c); end
                 end
               end
               begin
                 automatic int a = 2;
                 begin
                   automatic int b = 20;
                   begin automatic int c = 200; $display("R B %0d %0d %0d", a, b, c); end
                 end
               end
             end
           endmodule"#,
        &["R A 1 10 100", "R B 2 20 200"],
    );
}

/// Dynamic storage at both levels — strings and dynamic arrays are the family whose
/// heaps must not be shared. iverilog prints `R A x p 2` / `R B y q 3`.
#[test]
fn dynamic_storage_at_two_levels() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic string fd = "x";
                 begin
                   automatic string s = "p";
                   automatic byte   m [];
                   m = new[2];
                   $display("R A %s %s %0d", fd, s, m.size());
                 end
               end
               begin
                 automatic string fd = "y";
                 begin
                   automatic string s = "q";
                   automatic byte   m [];
                   m = new[3];
                   $display("R B %s %s %0d", fd, s, m.size());
                 end
               end
             end
           endmodule"#,
        &["R A x p 2", "R B y q 3"],
    );
}

/// The report's struct sub-case, which produced the "this one is static" claim about a
/// declaration the user spelled `automatic`. Both the message and the rejection are gone.
#[test]
fn struct_member_locals_at_two_levels() {
    runs(
        r#"module t;
             typedef struct { string msg_hex; int n; } rsp_t;
             initial begin
               begin
                 automatic int fd = 1;
                 begin
                   automatic rsp_t  r;
                   automatic string scen_name;
                   r.msg_hex = "aa"; r.n = fd; scen_name = "s1";
                   $display("R A %s %0d %s", r.msg_hex, r.n, scen_name);
                 end
               end
               begin
                 automatic int fd = 2;
                 begin
                   automatic rsp_t  r;
                   automatic string scen_name;
                   r.msg_hex = "bb"; r.n = fd; scen_name = "s2";
                   $display("R B %s %0d %s", r.msg_hex, r.n, scen_name);
                 end
               end
             end
           endmodule"#,
        &["R A aa 1 s1", "R B bb 2 s2"],
    );
}

/// The walker shape the 17 real sites came from: a `foreach` over a table, an outer
/// per-iteration local, and an inner block of per-record locals — twice.
#[test]
fn table_driven_walker_shape() {
    runs(
        r#"module t;
             string files [2] = '{"a", "b"};
             initial begin
               foreach (files[fi]) begin
                 automatic int fd = fi + 1;
                 begin
                   automatic int    n = 0;
                   automatic string scen_name;
                   automatic byte   msg [];
                   msg = new[fd]; n += msg.size(); scen_name = files[fi];
                   $display("R P %s %0d %0d", scen_name, fd, n);
                 end
               end
               foreach (files[fi]) begin
                 automatic int fd = fi + 10;
                 begin
                   automatic int    n = 0;
                   automatic string scen_name;
                   automatic byte   msg [];
                   msg = new[fd]; n += msg.size(); scen_name = files[fi];
                   $display("R Q %s %0d %0d", scen_name, fd, n);
                 end
               end
             end
           endmodule"#,
        &["R P a 1 1", "R P b 2 2", "R Q a 10 10", "R Q b 11 11"],
    );
}

/// SOUNDNESS PIN. A name declared at an outer level AND again at an inner level of the
/// SAME tree is SHADOWING, not two disjoint blocks — a distinct rule that stays. vita
/// cannot resolve the outer binding through the flatten, so it must stay loud rather
/// than silently pick one of the two.
#[test]
fn inner_shadowing_the_enclosing_same_name_stays_loud() {
    let (o, ok) = run(r#"module t;
             initial begin
               begin
                 automatic int v = 1;
                 begin automatic int v = 9; $display("R A %0d", v); end
                 $display("R A outer %0d", v);
               end
               begin
                 automatic int v = 2;
                 begin automatic int v = 8; $display("R B %0d", v); end
                 $display("R B outer %0d", v);
               end
             end
           endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "expected E3009, got:\n{o}");
}
