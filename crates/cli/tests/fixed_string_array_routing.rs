//! T1: a fixed `string` array routes to the DYNAMIC-array representation, which is what
//! gives it a RUNTIME index and `foreach` — for ANY constant declared bounds
//! (`s[2]`, `s[1:3]`, `s[3:1]`, `s[2][3]`, `s[1:2][2:1]`, …).
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
fn multi_dim_foreach_walks_every_dimension() {
    // T1-3. The parser desugars `foreach(s[i,j])` to `.first/.next(idx, DIM)`; the
    // dimension tag resolves against the ROUTED geometry, which the flat-word-store
    // descriptors (`array_dims`) do not describe because a routed array is a `DynArray`
    // net. iverilog: 00:a 01:b 10:c 11:d.
    assert_eq!(
        run("module m; string s[2][2];\n\
             initial begin s[0][0]=\"a\"; s[0][1]=\"b\"; s[1][0]=\"c\"; s[1][1]=\"d\";\n\
               foreach(s[i,j]) $write(\"%0d%0d:%s \", i, j, s[i][j]); $display(\"\"); $finish; end\n\
             endmodule\n"),
        "00:a 01:b 10:c 11:d \n"
    );
    // …and the walk follows each dim's own bounds and direction, not the flat order.
    // iverilog: 21 22 11 12 (dim 1 descending `[2:1]`, dim 2 ascending `[1:2]`).
    assert_eq!(
        run("module m; string s[2:1][1:2];\n\
             initial begin foreach(s[i,j]) $write(\"%0d%0d \", i, j); $display(\"\"); $finish; end\n\
             endmodule\n"),
        "21 22 11 12 \n"
    );
}

#[test]
fn multi_dim_non_zero_base() {
    // T1-5. iverilog: aa dd, then the foreach index pairs.
    assert_eq!(
        run("module m; string s[1:2][1:2];\n\
             initial begin s[1][1]=\"aa\"; s[2][2]=\"dd\";\n\
               $display(\"%s %s\", s[1][1], s[2][2]);\n\
               foreach(s[i,j]) $write(\"%0d%0d \", i, j); $display(\"\"); $finish; end\n\
             endmodule\n"),
        "aa dd\n11 12 21 22 \n"
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
fn a_multi_dim_init_pattern_must_match_the_declared_nesting() {
    // T1-4. The nested `'{'{…},'{…}}` is walked in lockstep with the declared dims, so
    // every level must hold EXACTLY that dim's element count and only the innermost
    // level holds values. Each malformed shape below is rejected by iverilog too
    // ("Unpacked array assignment pattern expects N element(s)"), so these are the
    // oracle's own boundaries, not a vita limitation.
    //
    // The FLAT form is the load-bearing one: it is the shape a 1-D expansion would
    // have accepted (2 outer elements on a `[2][2]`), and letting it through assigned
    // an assignment-pattern into a string element — four empty strings at exit 0.
    for init in [
        "'{\"a\",\"b\",\"c\",\"d\"}", // flat: right total, wrong nesting
        "'{'{\"a\",\"b\"},'{\"c\"}}", // short inner level
        "'{'{\"a\",\"b\"}}",          // short outer level
    ] {
        assert!(
            loud(&format!(
                "module m; string s[2][2] = {init};\n\
                 initial begin $display(\"%s\", s[0][0]); $finish; end\n\
                 endmodule\n"
            )),
            "init: {init}"
        );
    }
}

#[test]
fn a_multi_dim_init_pattern_fills_row_major_in_the_declared_direction() {
    // T1-4. iverilog: `[a][d]` and `[a][e]`. The second case is non-square, so a
    // transposed flatten would print `[a][d]` — the shape a square array cannot
    // distinguish. The first mixes directions per axis (`[2:1]` descending, `[1:2]`
    // ascending), which is the pattern-order rule (IEEE §10.9.1, left bound first)
    // applied per dimension rather than to the flat container.
    assert_eq!(
        run(
            "module m; string s[2:1][1:2] = '{'{\"a\",\"b\"},'{\"c\",\"d\"}};\n\
             initial begin $display(\"[%s][%s]\", s[2][1], s[1][2]); $finish; end\n\
             endmodule\n"
        ),
        "[a][d]\n"
    );
    assert_eq!(
        run(
            "module m; string s[2][3] = '{'{\"a\",\"b\",\"c\"},'{\"d\",\"e\",\"f\"}};\n\
             initial begin $display(\"[%s][%s]\", s[0][0], s[1][1]); $finish; end\n\
             endmodule\n"
        ),
        "[a][e]\n"
    );
}

// ── what must NOT change ─────────────────────────────────────────────────────

#[test]
fn a_non_zero_base_keeps_its_declared_index_space() {
    // T1-1. The container is still a flat `0..n-1` block; what changed is that the
    // declared→flat map is now APPLIED (`idx - lo` at every access) instead of assumed
    // to be the identity. iverilog: `aa cc`, then `aa`, then `1:aa 2:bb 3:cc`.
    assert_eq!(
        run("module m; string s[1:3]; int k;\n\
             initial begin s[1]=\"aa\"; s[2]=\"bb\"; s[3]=\"cc\";\n\
               k=1; $display(\"%s %s|%s\", s[1], s[3], s[k]);\n\
               foreach(s[j]) $write(\"%0d:%s \", j, s[j]); $display(\"\"); $finish; end\n\
             endmodule\n"),
        "aa cc|aa\n1:aa 2:bb 3:cc \n"
    );
}

#[test]
fn a_descending_declaration_walks_high_to_low() {
    // T1-2. The storage order is an arbitrary bijection (`idx - lo` either way), so the
    // direction shows up in ONE place: `foreach` traverses the declared bounds
    // left-to-right (IEEE §12.7.3), which for `[3:1]` is 3, 2, 1. iverilog agrees.
    assert_eq!(
        run("module m; string s[3:1]; int k;\n\
             initial begin s[1]=\"aa\"; s[2]=\"bb\"; s[3]=\"cc\";\n\
               k=1; $display(\"%s %s|%s\", s[1], s[3], s[k]);\n\
               foreach(s[j]) $write(\"%0d:%s \", j, s[j]); $display(\"\"); $finish; end\n\
             endmodule\n"),
        "aa cc|aa\n3:cc 2:bb 1:aa \n"
    );
}

#[test]
fn an_index_outside_the_declared_range_is_not_silently_remapped() {
    // The normalization is `idx - lo`, so an under-index on `[1:3]` would map to
    // word -1. That must NOT wrap into a valid element: it warns (W4020) and reads
    // empty, matching iverilog's `[ ]` for an out-of-bounds element.
    let (out, ok, err) = compile(
        "module m; string s[1:3];\n\
         initial begin s[1]=\"aa\"; $display(\"[%s][%s]\", s[0], s[4]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected a warn, not a reject");
    assert_eq!(out, "[ ][ ]\n");
    assert!(err.contains("W4020"), "expected the OOB warn, got:\n{err}");
}

#[test]
fn a_negative_bound_is_not_routed() {
    // `flatten_word` carries `lo` as `u32`, so a negative base has no map — and the
    // GENERAL unpacked-array path shares that limit (`int a[-1:1]` drops the `-1`
    // element with an E4002 at HEAD). Declining keeps the string form on the
    // per-element-net path, where a CONST index still resolves, rather than inheriting
    // a known-broken mapping. iverilog: `aa cc`.
    assert_eq!(
        run("module m; string s[-1:1];\n\
             initial begin s[-1]=\"aa\"; s[1]=\"cc\"; $display(\"%s %s\", s[-1], s[1]); $finish; end\n\
             endmodule\n"),
        "aa cc\n"
    );
    // …and a runtime index there is still honestly loud, not silently zero-based.
    assert!(loud(
        "module m; string s[-1:1]; int k;\n\
         initial begin k=1; s[k]=\"aa\"; $display(\"%s\", s[1]); $finish; end\n\
         endmodule\n"
    ));
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
