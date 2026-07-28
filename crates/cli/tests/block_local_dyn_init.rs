//! round-19 BL2 + BL3: a PROCEDURAL-BLOCK-LOCAL dynamic-storage (`dyn array` /
//! string dyn array / queue) declared `automatic` WITH a `'{…}` / `{…}` / `new[]`
//! initializer was rejected E3009 ("…per-entry lifetime differs from static…") by
//! the block-local per-entry-lifetime gate. It is now supported: the decl-init
//! EXPANSION (`d = new[N]; d[i] = e;` / `q.push_back(e)`) is re-emitted at BLOCK
//! ENTRY (IEEE §6.21 per-entry lifetime) on the one flattened handle net — mirroring
//! the already-working STATIC single-block dyn `'{…}` init and the mid-body
//! `new[]`+element-write expansion.
//!
//! BL3 (same name declared `automatic` in ≥2 disjoint blocks) rides the pre-existing
//! `$blk$<span>` distinct-net scoping (`compute_scoped_block_locals`) — each block
//! gets its OWN net and re-inits it at entry, so no coalesce-guard relaxation is
//! needed.
//!
//! No external oracle — iverilog 13.0 / verilator reject `automatic` lifetime
//! override. Reference behavior is the ALREADY-WORKING boundary: a STATIC
//! single-block dyn `'{…}` init runs today (P2), and mid-body `new[]`+writes run
//! today (P8/P9). BL2/BL3's gap was ONLY the `automatic` + initializer form.
//!
//! correct-or-loud: under-fork dyn (concurrency — a module process has no
//! per-activation frame arena), assoc / multi-dim dyn, and a same-name dyn COALESCE
//! that does NOT qualify for `$blk$` scoping (would leak the first block's heap)
//! stay loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (combined stdout+stderr, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bldi_{}_{n}", std::process::id()));
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

/// Loud with the specific per-entry-lifetime code (E3009).
fn loud3009(src: &str) -> bool {
    let (o, ok) = run(src);
    !ok && o.contains("E3009")
}

/// Loud with any error (for shapes whose exact code may vary — multi-dim / assoc).
fn loud(src: &str) -> bool {
    !run(src).1
}

// ── BL2: single-block dyn-storage `'{…}` init (loud → correct-support) ───────

#[test]
fn single_block_string_dyn_init() {
    // The BL2 repro: a string dynamic array declared `automatic` with a `'{…}` init.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic string files[] = '{\"a.rsp\", \"b.rsp\"};\n\
             if (files.size() == 2 && files[0] == \"a.rsp\" && files[1] == \"b.rsp\")\n\
               $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(o.contains("PASS"), "string dyn init did not populate:\n{o}");
}

#[test]
fn single_block_byte_dyn_init() {
    // The BL2 repro: a byte dynamic array declared `automatic` with a `'{…}` init.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic byte msg[] = '{8'h0, 8'h1};\n\
             if (msg.size() == 2 && msg[0] == 8'h0 && msg[1] == 8'h1)\n\
               $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(o.contains("PASS"), "byte dyn init did not populate:\n{o}");
}

// ── BL3: same-name dyn in disjoint blocks (rides `$blk$` scoping) ────────────

#[test]
fn same_name_dyn_init_two_blocks() {
    // The BL3 repro: `msg` declared `automatic byte msg[] = '{…}` in TWO disjoint
    // blocks with DIFFERENT sizes. Each block gets its own `$blk$` net and re-inits
    // at entry, so each reads its OWN size (2 then 3), not a leaked/aliased value.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic byte msg[] = '{8'd10, 8'd11};\n\
             if (msg.size() == 2 && msg[0] == 8'd10) $display(\"A\");\n\
           end\n\
           begin\n\
             automatic byte msg[] = '{8'd20, 8'd21, 8'd22};\n\
             if (msg.size() == 3 && msg[2] == 8'd22) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    let a = o.find("A");
    let p = o.find("PASS");
    assert!(a.is_some() && p.is_some(), "both blocks must fire:\n{o}");
    assert!(a < p, "block 1 (A) must precede block 2 (PASS):\n{o}");
}

#[test]
fn same_name_dyn_three_scenarios() {
    // Models tb_hash_top's ×49 same-name `msg[]` blocks: three disjoint blocks, each
    // a different-size `'{…}`, each reading its own size. Each is a distinct `$blk$`
    // net re-init at entry.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin automatic byte msg[] = '{8'd1};              if (msg.size()==1) $display(\"S1\"); end\n\
           begin automatic byte msg[] = '{8'd1,8'd2};         if (msg.size()==2) $display(\"S2\"); end\n\
           begin automatic byte msg[] = '{8'd1,8'd2,8'd3};    if (msg.size()==3) $display(\"S3\"); end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(
        o.contains("S1") && o.contains("S2") && o.contains("S3"),
        "each same-name block must read its own size:\n{o}"
    );
}

// ── BL2 per-entry semantics: re-init each block entry ────────────────────────

#[test]
fn dyn_init_in_loop_reinits() {
    // A dyn `'{…}` init inside a LOOP body re-runs every iteration (§6.21). The init
    // reads the loop var `i`, so a once-at-t0 static init could not produce the right
    // per-iteration contents — `seen` reaches 3 only with per-entry re-init.
    let (o, ok) = run("module top;\n\
         int seen;\n\
         initial begin\n\
           seen = 0;\n\
           for (int i = 0; i < 3; i++) begin\n\
             automatic byte m[] = '{i, i, i};\n\
             if (m.size() == 3 && m[0] == i && m[2] == i) seen = seen + 1;\n\
           end\n\
           if (seen == 3) $display(\"PASS seen=%0d\", seen);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(
        o.contains("PASS seen=3"),
        "loop did not re-init per entry:\n{o}"
    );
}

#[test]
fn dyn_init_by_value() {
    // Adversarial value-semantics (§7.9 deep copy): the per-entry `'{…}` init gives a
    // real independent heap. A whole-handle copy `cpy = msg` snapshots it; a later
    // `msg[0] = 99` must NOT show through to cpy (independent heaps, not an alias).
    let (o, ok) = run("module top;\n\
         byte cpy[];\n\
         initial begin\n\
           begin\n\
             automatic byte msg[] = '{8'd5, 8'd6};\n\
             cpy = msg;\n\
             msg[0] = 8'd99;\n\
             if (cpy[0] == 8'd5 && cpy[1] == 8'd6 && msg[0] == 8'd99 && msg[1] == 8'd6)\n\
               $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(o.contains("PASS"), "deep-copy value semantics broke:\n{o}");
}

// ── bonus: queue block-local `'{…}` init (+ per-entry clear on re-entry) ──────

#[test]
fn queue_block_local_dyn_init() {
    // A queue block-local `'{…}` init expands to `q.push_back(e)`; the per-entry
    // path clears (`q.delete()`) before pushing so a re-entry does not accumulate.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic int q[$] = '{1, 2, 3};\n\
             if (q.size() == 3 && q[0] == 1 && q[2] == 3) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(o.contains("PASS"), "queue dyn init did not populate:\n{o}");
}

#[test]
fn queue_block_local_reinits_in_loop() {
    // A queue per-entry re-init MUST clear first (push_back appends). Without the
    // clear, iteration 2 would see size 4 and iteration 3 size 6 — `ok` reaches 3
    // only if each entry resets to exactly 2 elements.
    let (o, ok) = run("module top;\n\
         int ok;\n\
         initial begin\n\
           ok = 0;\n\
           for (int k = 0; k < 3; k++) begin\n\
             automatic int q[$] = '{k, k};\n\
             if (q.size() == 2 && q[0] == k) ok = ok + 1;\n\
           end\n\
           if (ok == 3) $display(\"PASS ok=%0d\", ok);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("PASS ok=3"),
        "queue did not clear on re-entry (accumulated):\n{o}"
    );
}

// ── regression: the ALREADY-WORKING boundaries must stay working ─────────────

#[test]
fn static_single_block_dyn_still_works() {
    // P2: a STATIC (non-`automatic`) single-block dyn `'{…}` init runs today; the
    // BL2 change (automatic + per-entry) must not perturb it.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             byte msg[] = '{8'd0, 8'd1};\n\
             if (msg.size() == 2 && msg[1] == 8'd1) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "static single-block dyn broke:\n{o}");
    assert!(o.contains("PASS"), "static single-block dyn broke:\n{o}");
}

#[test]
fn ok_bl_samename_new() {
    // Same-name dyn using a mid-body `new[N]` (no decl-init) in two disjoint blocks —
    // rides the pre-existing `$blk$` scoping (distinct nets, each definitely-assigned).
    // Independent of the BL2/BL3 per-entry-init change; kept as a boundary pin so the
    // dyn per-entry recording does not perturb the no-init same-name new[] path.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic byte msg[];\n\
             msg = new[2];\n\
             msg[0] = 8'd1; msg[1] = 8'd2;\n\
             if (msg.size() == 2) $display(\"A\");\n\
           end\n\
           begin\n\
             automatic byte msg[];\n\
             msg = new[3];\n\
             if (msg.size() == 3) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(
        o.contains("A") && o.contains("PASS"),
        "same-name new[] blocks must each work:\n{o}"
    );
}

#[test]
fn nonfork_scalar_per_entry_still_works() {
    // The pre-existing family-D scalar per-entry path (`automatic int lim = 20`) must
    // stay working — the dyn extension shares the same `per_entry_block_locals` map.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic int lim = 20;\n\
             $display(\"lim=%0d\", lim);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "scalar per-entry broke:\n{o}");
    assert!(o.contains("lim=20"), "scalar per-entry broke:\n{o}");
}

// ── correct-or-loud (MUST stay loud) ─────────────────────────────────────────

#[test]
fn a_single_spawn_fork_arm_gets_its_dynamic_init() {
    // §4.5.248 supersedes the old "under a fork ⇒ loud" rule here too. A `fork` the
    // process reaches ONCE gives each arm exactly one activation, so the one flattened
    // handle holds one array — the per-entry expansion (`m = new[N]; m[i] = e;`) at
    // block entry is as correct here as in a straight-line block.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic byte m[] = '{8'd7, 8'd9};\n\
               $display(\"m=%0d %0d %0d\", m.size(), m[0], m[1]);\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("m=2 7 9"),
        "dyn per-entry init in a fork arm:\n{o}"
    );

    // A queue in the same position — its expansion clears, then push_backs.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int q[$] = '{1, 2, 3};\n\
               $display(\"q=%0d %0d\", q.size(), q[2]);\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("q=3 3"),
        "queue per-entry init in a fork arm:\n{o}"
    );
}

#[test]
fn a_multi_spawn_fork_arm_dyn_local_stays_loud() {
    // The spawn point is inside a loop, so arm N+1 starts while N may still be live —
    // two activations, one heap handle. This is what the fork exclusion is really for.
    assert!(loud3009(
        "module top;\n\
         initial begin\n\
           for (int i = 0; i < 2; i++)\n\
             fork\n\
               begin\n\
                 automatic byte m[] = '{8'd0, 8'd1};\n\
                 #1 $display(\"SHOULD_NOT %0d\", m.size());\n\
               end\n\
             join_none\n\
           #5 $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn same_name_dyn_locals_across_blocks_do_not_leak() {
    // The loud this replaces existed to stop a LEAK: block 2 declares the same name
    // STATIC and would coalesce onto block 1's persistent dyn heap, reading its
    // elements. §4.5.249 gives each disjoint block's dynamic local its own `$blk$` net,
    // so the leak is gone by construction — block 2 sees the empty array it declared,
    // which is exactly what the loud was protecting.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic byte msg[] = '{8'd1, 8'd2};\n\
             $display(\"A=%0d\", msg.size());\n\
           end\n\
           begin\n\
             byte msg[];\n\
             $display(\"B=%0d\", msg.size());\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("A=2"), "block 1 keeps its elements:\n{o}");
    assert!(o.contains("B=0"), "block 2 must NOT see them:\n{o}");
}

#[test]
fn a_same_name_dyn_local_with_an_initializer_stays_loud() {
    // The `$blk$` path skips the decl-init collector, so a SCOPED static
    // `byte m[] = '{…}` would come up EMPTY. Measured, not assumed — the first cut of
    // §4.5.249 printed `size()==0` here. So an initializer-bearing STATIC dynamic local
    // is excluded from the widening and keeps the loud, which now names the identifier
    // and states the actual rule.
    assert!(loud3009(
        "module top;\n\
         initial begin\n\
           begin byte m[] = '{8'd1, 8'd2}; $display(\"A=%0d\", m.size()); end\n\
           begin byte m[]; $display(\"B=%0d\", m.size()); end\n\
           $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn multidim_dyn_block_local_stays_loud() {
    // A MULTI-dim dyn (`byte m[][]`) is excluded (`unpacked.len() != 1`) — stays loud.
    assert!(loud(
        "module top;\n\
         initial begin\n\
           begin\n\
             automatic byte m[][] = '{'{8'd1}, '{8'd2}};\n\
             if (m.size() == 2) $display(\"SHOULD_NOT\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn assoc_block_local_stays_loud() {
    // An associative array (`Dim::Assoc`) is excluded from the per-entry-dyn path —
    // stays loud (it has no `'{…}` decl-init expansion).
    assert!(loud(
        "module top;\n\
         initial begin\n\
           begin\n\
             automatic int a[string] = '{\"x\": 1, \"y\": 2};\n\
             if (a.num() == 2) $display(\"SHOULD_NOT\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n"
    ));
}

// ── §4.5.249: same-named DYNAMIC locals in disjoint blocks get distinct storage ──

#[test]
fn same_named_dyn_locals_are_distinct_variables_whatever_their_lifetime() {
    // `$blk$` scoping used to require BOTH declarations be `automatic`, so any static
    // one in the group left the whole name loud — the round-20 report measured exactly
    // this ("static ↔ automatic mixed → loud") as one trigger of its 81 unisolated
    // diagnostics. Two same-named dynamic locals in DISJOINT blocks are two distinct
    // variables in IEEE regardless of lifetime, and the pair was loud whenever either
    // side was static, so scoping them is a pure loud → support move.
    //
    // The 3-block dynamic-array form below is pinned against LIVE iverilog 13.0
    // (`A1 B2 C0`).
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin byte m[]; m = new[1]; $display(\"A=%0d\", m.size()); end\n\
           begin byte m[]; m = new[2]; $display(\"B=%0d\", m.size()); end\n\
           begin byte m[]; $display(\"C=%0d\", m.size()); end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("A=1") && o.contains("B=2") && o.contains("C=0"),
        "three distinct heaps:\n{o}"
    );

    // Queue and associative array, same shape — the third block must see nothing.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin int q[$]; q.push_back(1); q.push_back(2); $display(\"Q=%0d\", q.size()); end\n\
           begin int q[$]; $display(\"R=%0d\", q.size()); end\n\
           begin int aa[int]; aa[3] = 7; $display(\"S=%0d\", aa.num()); end\n\
           begin int aa[int]; $display(\"T=%0d\", aa.num()); end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    for want in ["Q=2", "R=0", "S=1", "T=0"] {
        assert!(o.contains(want), "expected `{want}`; got:\n{o}");
    }
}

#[test]
fn a_scoped_static_dyn_local_does_not_shadow_a_module_net() {
    // The `module_names` exclusion still holds: a name that ALSO names a module-scope
    // net is not scoped, so the module net keeps its single identity.
    let (o, ok) = run("module top;\n\
         byte m [];\n\
         initial begin\n\
           m = new[3];\n\
           begin $display(\"A=%0d\", m.size()); end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("A=3"), "the module net:\n{o}");
}
