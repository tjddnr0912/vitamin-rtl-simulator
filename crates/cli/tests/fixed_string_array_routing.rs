//! T1: a ZERO-BASED ASCENDING fixed `string` array routes to the DYNAMIC-array
//! representation, which is what gives it a RUNTIME index and `foreach`.
//!
//! The per-element-net form (`string s[2]` → nets `s$sae$0`, `s$sae$1`) cannot express
//! a runtime index at all — the index would have to select among N distinct nets. The
//! dynamic form is one `DynArray` handle whose elements live in the engine heap, where
//! an index is an ordinary runtime value.
//!
//! Oracle: iverilog, live, on every shape below except the two noted as hand-IEEE.
//!
//! Why unifying the two storage classes is a climb and not a trade: capability parity
//! was MEASURED across 23 shapes (decl-init, const index, byte select, element
//! `.len()`/`.getc()`/`.toupper()`/`.substr()`, element-to-element copy, function
//! argument, ternary, `$sformatf`, `case`, compare, concat, empty read, …) and the two
//! agree on every one, with dyn additionally answering runtime index, `foreach`,
//! runtime write and `.size()`. Before §4.5.220 that was NOT true — the dyn element
//! byte select read a silent 0 where fixed gave 119 — so routing then would have traded
//! one silent-wrong for another.
//!
//! Two regressions found by adversarial review during this slice are pinned below, both
//! caused by the routed array taking its own BARE name (`namespace_*`, `alias_*`).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn compile(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fsar_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            }),
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = compile(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

fn loud(src: &str) -> bool {
    !compile(src).1
}

// ── the two gaps this slice closes ───────────────────────────────────────────

#[test]
fn runtime_index_read() {
    // iverilog: bb
    let out = run("module m; string s[2]; int i;\n\
        initial begin s[0]=\"aa\"; s[1]=\"bb\"; i=1; $display(\"%s\", s[i]); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "bb\n");
}

#[test]
fn runtime_index_write() {
    // iverilog: e0 e1 e2 — a loop that writes every element through a runtime index.
    let out = run("module m; string s[3]; int k;\n\
        initial begin for(k=0;k<3;k=k+1) s[k]=\"xx\"; \
        $display(\"%s %s %s\", s[0], s[1], s[2]); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "xx xx xx\n");
}

#[test]
fn foreach_walks_declared_indices() {
    // iverilog: 0 aa / 1 bb
    let out = run("module m; string s[2];\n\
        initial begin s[0]=\"aa\"; s[1]=\"bb\"; \
        foreach(s[j]) $display(\"%0d %s\", j, s[j]); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "0 aa\n1 bb\n");
}

#[test]
fn runtime_index_keeps_the_string_domain_in_a_concat() {
    // iverilog: [aa!][<aa>] — the element must concatenate as BYTES, not as the packed
    // value a width-0 handle would give.
    let out = run("module m; string s[2]; int k;\n\
        initial begin s[0]=\"aa\"; k=0; \
        $display(\"[%s][%s]\", {s[k],\"!\"}, {\"<\",s[k],\">\"}); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "[aa!][<aa>]\n");
}

#[test]
fn byte_select_through_a_runtime_index() {
    // iverilog: 119 ('w'). The §4.5.220 dyn byte select is what makes this survive the
    // storage change; without it the routing would have regressed this to a silent 0.
    let out = run("module m; string s[2]; int k;\n\
        initial begin s[0]=\"wa\"; k=0; $display(\"%0d\", s[k][0]); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "119\n");
}

#[test]
fn decl_init_pattern_still_works_and_is_indexable() {
    // iverilog: aa bb. The `'{…}` expansion is unchanged — the decl validates the
    // element COUNT and the collectors expand it; only the storage moved.
    let out = run("module m; string s[2] = '{\"aa\",\"bb\"}; int k;\n\
        initial begin k=1; $display(\"%s %s\", s[0], s[k]); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "aa bb\n");
}

#[test]
fn block_local_declaration_routes_too() {
    // The two decl-init collectors (module scope / block-local) must stay in step —
    // they drifted once before and silently emptied a block-local array.
    let out = run("module m;\n\
        initial begin string s[2] = '{\"aa\",\"bb\"}; int k; k=1; $display(\"%s\", s[k]); end\n\
        initial #1 $finish;\n\
        endmodule\n");
    assert_eq!(out, "bb\n");
}

#[test]
fn declaration_forms_that_are_zero_based_ascending() {
    for decl in ["string s[2];", "string s[0:1];"] {
        let out = run(&format!(
            "module m; {decl} int k;\n\
             initial begin s[0]=\"aa\"; s[1]=\"bb\"; k=1; $display(\"%s\", s[k]); $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(out, "bb\n", "decl: {decl}");
    }
}

#[test]
fn size_of_a_fixed_string_array() {
    // hand-IEEE (§7.4.2 — a fixed-size array supports `.size()`); iverilog rejects the
    // call on a fixed array, so there is no oracle. The declared length is the answer.
    assert_eq!(
        run("module m; string s[3];\n\
             initial begin $display(\"%0d\", s.size()); $finish; end\n\
             endmodule\n"),
        "3\n"
    );
}

// ── T1-5: multi-dimensional ──────────────────────────────────────────────────

#[test]
fn multi_dim_element_read_and_write() {
    // iverilog: aa dd. `string s[2][2]` is ONE flat 4-element container; `s[i][j]`
    // flattens row-major at each access. Both funnels bottom out on an `Ident` base,
    // which a nested select never is — so read and write each walk the chain, and they
    // must decline on exactly the same conditions or the two would land on different
    // elements.
    assert_eq!(
        run("module m; string s[2][2];\n\
             initial begin s[0][0]=\"aa\"; s[1][1]=\"dd\"; \
             $display(\"%s %s\", s[0][0], s[1][1]); $finish; end\n\
             endmodule\n"),
        "aa dd\n"
    );
}

#[test]
fn multi_dim_row_major_order_is_observable() {
    // A non-square shape is what distinguishes row-major from column-major: with
    // `s[2][3]` a transposed flatten would collide `s[0][2]` with `s[1][0]`. Writing
    // every cell and reading three of them back pins the order. iverilog: x x LAST.
    assert_eq!(
        run("module m; string s[2][3]; int i,j;\n\
             initial begin\n\
               for(i=0;i<2;i=i+1) for(j=0;j<3;j=j+1) s[i][j]=\"x\";\n\
               s[1][2]=\"LAST\";\n\
               $display(\"%s %s %s\", s[0][0], s[1][1], s[1][2]); $finish; end\n\
             endmodule\n"),
        "x x LAST\n"
    );
}

#[test]
fn three_dimensional() {
    // iverilog: A Z — the flatten is N-D, not a special case for two.
    assert_eq!(
        run("module m; string s[2][2][2];\n\
             initial begin s[0][0][0]=\"A\"; s[1][1][1]=\"Z\"; \
             $display(\"%s %s\", s[0][0][0], s[1][1][1]); $finish; end\n\
             endmodule\n"),
        "A Z\n"
    );
}

#[test]
fn a_partial_index_of_a_multi_dim_array_is_loud() {
    // REGRESSION GUARD. A single index on a 2-D array is a partial index — it selects a
    // whole row, which has no value surface (iverilog rejects the source). The flat
    // container would otherwise take the ROW number as an ELEMENT number: measured, the
    // read printed an empty string at exit 0. Raised at BOTH funnels, or the write would
    // stay silent while the read was loud.
    assert!(loud(
        "module m; string s[2][2];\n\
        initial begin s[0][0]=\"aa\"; $display(\"%s\", s[0]); $finish; end\n\
        endmodule\n"
    ));
    assert!(loud(
        "module m; string s[2][2];\n\
        initial begin s[0]=\"aa\"; $display(\"done\"); $finish; end\n\
        endmodule\n"
    ));
}

#[test]
fn a_multi_dim_declaration_that_does_not_qualify_stays_loud() {
    // Every dimension must be zero-based ascending, because the container is FLAT: a
    // non-conforming axis would be renumbered silently. `string s[1:2][1:2]` is a
    // capability gap (iverilog runs it) and an honest reject — not a wrong answer.
    // A multi-dim `'{…}` decl-init is loud for the same reason: the expansion speaks one
    // dimension, and filling the first N elements of the flat container is not the same
    // array.
    assert!(loud(
        "module m; string s[1:2][1:2];\n\
        initial begin s[1][1]=\"aa\"; $display(\"%s\", s[1][1]); $finish; end\n\
        endmodule\n"
    ));
    assert!(loud(
        "module m; string s[2][2] = '{\"a\",\"b\",\"c\",\"d\"};\n\
        initial begin $display(\"%s\", s[0][0]); $finish; end\n\
        endmodule\n"
    ));
}

// ── what must NOT change ─────────────────────────────────────────────────────

#[test]
fn non_zero_base_and_descending_are_not_routed() {
    // The dyn representation numbers elements 0..n-1 and `foreach` walks them in that
    // order, so routing `[1:3]` would silently RENUMBER the index space (iverilog's
    // `foreach` over `a[1:3]` yields 1,2,3) and `[3:1]` would additionally re-order it.
    // Those keep the per-element-net path: const index still works, runtime index is
    // still honestly loud.
    for decl in ["string s[1:3];", "string s[3:1];"] {
        let out = run(&format!(
            "module m; {decl}\n\
             initial begin s[1]=\"aa\"; s[3]=\"cc\"; $display(\"%s %s\", s[1], s[3]); $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(out, "aa cc\n", "decl: {decl}");
        assert!(
            loud(&format!(
                "module m; {decl} int k;\n\
                 initial begin k=1; s[k]=\"aa\"; $display(\"%s\", s[1]); $finish; end\n\
                 endmodule\n"
            )),
            "decl: {decl}"
        );
    }
}

#[test]
fn a_fixed_string_array_is_not_resizable() {
    // The routed net is fixed-SIZE storage that merely happens to be dyn-backed. Left
    // ungated, the routing would turn today's honest reject into a SILENT resize — a
    // descent. Even a same-SIZE `new[]` is rejected: the operation itself is wrong.
    for n in ["5", "2"] {
        assert!(
            loud(&format!(
                "module m; string s[2];\n\
                 initial begin s[0]=\"a\"; s = new[{n}]; $display(\"x\"); $finish; end\n\
                 endmodule\n"
            )),
            "new[{n}]"
        );
    }
    // A genuine dynamic array is of course still resizable.
    assert_eq!(
        run("module m; string s[];\n\
             initial begin s=new[2]; s[0]=\"a\"; s=new[5]; \
             $display(\"sz=%0d\", s.size()); $finish; end\n\
             endmodule\n"),
        "sz=5\n"
    );
}

#[test]
fn element_count_mismatch_in_a_decl_init_is_loud() {
    // iverilog rejects it too. Without the count check the routed array would silently
    // become 2 elements long where 3 were declared.
    assert!(loud(
        "module m; string s[3] = '{\"aa\",\"bb\"};\n\
        initial begin $display(\"%s\", s[0]); $finish; end\n\
        endmodule\n"
    ));
}

// ── regressions the adversarial review caught (both from the bare name) ──────

#[test]
fn namespace_a_non_string_block_local_of_the_same_name_still_runs() {
    // REGRESSION GUARD. v1 flattens a block-local onto a module net by BARE NAME. The
    // first version of this routing registered the array under its declared name, so a
    // block-local `logic [7:0] sa` collided with it and hit the dynamic-storage reject
    // — two designs iverilog runs, and vita ran correctly BEFORE the routing, went loud.
    //
    // The fix is to keep the array's storage off its own name (`sa$sad`), exactly as the
    // per-element-net form kept it on `sa$sae$i`. iverilog: blk=aa bb / mod=ZZ YY len=2.
    let out = run("module top;\n\
           string sa[2];\n\
           initial begin\n\
             logic [7:0] sa[2];\n\
             sa[0]=8'hAA; sa[1]=8'hBB;\n\
             $display(\"blk=%0h %0h\", sa[0], sa[1]);\n\
           end\n\
           initial begin sa[0]=\"ZZ\"; sa[1]=\"YY\";\n\
             #1 $display(\"mod=%s %s len=%0d\", sa[0], sa[1], sa[1].len()); end\n\
         endmodule\n");
    assert_eq!(out, "blk=aa bb\nmod=ZZ YY len=2\n");
}

#[test]
fn alias_a_string_block_local_of_the_same_name_is_still_loud() {
    // REGRESSION GUARD. §4.5.218 made this loud because it silently ALIASED. Routing
    // moved the array off `string_array_elems`, which stopped the collision guard from
    // firing, and the alias came straight back in a NEW shape: the module's own
    // `sa[0]="zz"` and its read-back resolved through DIFFERENT resolvers (the write
    // reached the block-local scalar, the read the routed array), so iverilog's
    // `R=zz,yy` became a silent `R=,` at exit 0. The guard is now keyed on the storage,
    // not on one of the two representations.
    let (_, ok, err) = compile(
        "module t;\n\
           string sa[2];\n\
           initial begin : blk\n\
             string sa;\n\
             sa = \"AZ\";\n\
           end\n\
           initial begin sa[0]=\"zz\"; sa[1]=\"yy\"; #1 $display(\"R=%s,%s\", sa[0], sa[1]); end\n\
         endmodule\n",
    );
    assert!(!ok, "expected a loud reject");
    assert!(
        err.contains("collides with a string ARRAY"),
        "unexpected diagnostic:\n{err}"
    );
}

#[test]
fn an_inner_dynamic_local_still_shadows_the_module_array() {
    // The routed array resolves through a side map consulted BEFORE the symbol table,
    // so the shadowing order has to come from the scope walk rather than from that
    // ordering. A function-local `int sa[]` must still win inside the function while
    // the module `string sa[2]` stays intact outside it. iverilog: 42 mm.
    let out = run("module m; string sa[2];\n\
        function automatic int f(); int sa[]; sa = new[3]; sa[0]=42; return sa[0]; endfunction\n\
        initial begin sa[0]=\"mm\"; $display(\"%0d %s\", f(), sa[0]); $finish; end\n\
        endmodule\n");
    assert_eq!(out, "42 mm\n");
}

#[test]
fn each_instance_gets_its_own_array() {
    // The side map is keyed by FULLY-QUALIFIED name, so two instances of the same
    // module must not share one array. iverilog: 1:one,fix / 2:two,fix.
    let out = run("module sub#(parameter int ID=0); string s[2]; int k;\n\
          initial begin s[0]=(ID==1)?\"one\":\"two\"; s[1]=\"fix\"; k=0; \
          #1 $display(\"%0d:%s,%s\", ID, s[k], s[1]); end\n\
        endmodule\n\
        module m; sub #(1) u1(); sub #(2) u2(); initial begin #2 $finish; end endmodule\n");
    assert_eq!(out, "1:one,fix\n2:two,fix\n");
}

#[test]
fn a_generate_scope_declaration_is_not_routed() {
    // Routing needs the t0 var-init flush to drive the `new[n]` pre-size, and only the
    // module body and block-local scopes run it. A generate scope keeps the
    // per-element-net path verbatim — const index works there exactly as before.
    let out = run("module m; genvar g;\n\
        generate for(g=0;g<1;g=g+1) begin : blk\n\
          string s[2];\n\
          initial begin s[0]=\"aa\"; $display(\"%s\", s[0]); end\n\
        end endgenerate\n\
        initial #1 $finish;\n\
        endmodule\n");
    assert_eq!(out, "aa\n");
}
