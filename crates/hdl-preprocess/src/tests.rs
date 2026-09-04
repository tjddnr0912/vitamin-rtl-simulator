use super::*;
use std::path::{Path, PathBuf};

/// In-memory include shim. `files` is keyed by FULL virtual path (e.g.
/// "/virtual/sub/b.svh"). `resolve` joins `request` onto `current_dir` (then each
/// incdir), normalizes `.`/`..`, and looks the result up — so it honors the
/// IEEE nested-include rule and lets a test prove directory-relative resolution.
/// canon == the normalized joined path; its parent is the new file's `dir`.
struct MemReader {
    files: std::collections::BTreeMap<String, String>,
}
impl MemReader {
    /// Lexically normalize a path to a "/"-joined string (no fs access): drop
    /// "." components, pop on "..". Splits on BOTH separators — on Windows
    /// `Path::join` inserts `\`, which silently missed the '/'-keyed map
    /// (caught by the first 3-OS CI run: nested include NOT-FOUND on
    /// windows-latest only).
    fn norm(p: &Path) -> String {
        let mut parts: Vec<&str> = Vec::new();
        let lossy = p.to_string_lossy();
        for comp in lossy.split(['/', '\\']) {
            match comp {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other),
            }
        }
        format!("/{}", parts.join("/"))
    }
    fn try_key(&self, base: &Path, request: &str) -> Option<(String, PathBuf, String)> {
        let key = Self::norm(&base.join(request));
        self.files
            .get(&key)
            .map(|t| (key.clone(), PathBuf::from(&key), t.clone()))
    }
}
impl IncludeReader for MemReader {
    fn resolve(
        &self,
        request: &str,
        current_dir: &Path,
        incdirs: &[PathBuf],
    ) -> Result<(String, PathBuf, String), ()> {
        if let Some(hit) = self.try_key(current_dir, request) {
            return Ok(hit);
        }
        for d in incdirs {
            if let Some(hit) = self.try_key(d, request) {
                return Ok(hit);
            }
        }
        Err(())
    }
}

fn pp(src: &str) -> PpResult {
    preprocess_str(Path::new("/virtual"), "top.sv", src, &PreOpts::default())
}

/// PP-FANOUT-CAP (2026-06-22 audit): chained doubling macros
/// (`` `Mi = `Mi-1 `Mi-1 ``) expand to 2^N copies at depth N, so the depth
/// guard never trips and a ~30-line file OOMs (≈8 GiB at N=24, measured
/// 2.1 GiB at N=22). The cumulative-output budget must turn this into a loud
/// bounded error instead of unbounded materialization.
#[test]
fn macro_fanout_is_bounded_not_oom() {
    let mut src = String::from("`define M0 xx\n");
    for i in 1..=20 {
        src.push_str(&format!("`define M{i} `M{} `M{}\n", i - 1, i - 1));
    }
    // expand OUTSIDE a string literal so the substitution actually fires.
    src.push_str("module t; initial $display(\"%0d\", $bits({`M20})); endmodule\n");
    let opts = PreOpts {
        max_output_bytes: 64 * 1024, // small so the test is fast
        ..PreOpts::default()
    };
    let r = preprocess_str(Path::new("/virtual"), "top.sv", &src, &opts);
    assert!(
        r.has_errors(),
        "exponential fan-out must fail loud, not silently expand"
    );
    assert!(
        r.diags
            .iter()
            .any(|d| d.code == MsgCode::PpRecursiveMacro && d.message.contains("budget")),
        "expected the output-budget diagnostic, got {:?}",
        r.diags
    );
    // the materialized text must stay near the cap, not balloon to MBs.
    assert!(
        r.text.len() <= 64 * 1024 + 4096,
        "output must stay bounded by the budget, got {} bytes",
        r.text.len()
    );
}

#[test]
fn timescale_literal_parsing() {
    assert_eq!(
        parse_timescale("1ns/100ps"),
        Ok(TimeScale {
            unit_exp: -9,
            prec_exp: -10
        })
    );
    assert_eq!(
        parse_timescale("10ns/1ns"),
        Ok(TimeScale {
            unit_exp: -8,
            prec_exp: -9
        })
    );
    assert_eq!(
        parse_timescale(" 1us / 1ps "),
        Ok(TimeScale {
            unit_exp: -6,
            prec_exp: -12
        })
    );
    // precision coarser than unit → error
    assert!(parse_timescale("1ns/1us").is_err());
    // bad mantissa / unit
    assert!(parse_timescale("5ns/1ns").is_err());
    assert!(parse_timescale("1xs/1ns").is_err());
    assert!(parse_timescale("1ns").is_err());
}

#[test]
fn timescale_region_table_file_order() {
    let r =
        pp("`timescale 1ns/100ps\nmodule a; endmodule\n`timescale 10ns/1ns\nmodule b; endmodule\n");
    assert!(!r.has_errors(), "diags: {:?}", r.diags);
    assert_eq!(r.timescales.len(), 2);
    assert_eq!(
        r.timescales[0].1,
        TimeScale {
            unit_exp: -9,
            prec_exp: -10
        }
    );
    assert_eq!(
        r.timescales[1].1,
        TimeScale {
            unit_exp: -8,
            prec_exp: -9
        }
    );
    // the first region begins before `module a`, the second before `module b`.
    let a = r.text.find("module a").unwrap();
    let b = r.text.find("module b").unwrap();
    assert!(r.timescales[0].0 <= a && a < r.timescales[1].0);
    assert!(r.timescales[1].0 <= b);
}

#[test]
fn timescale_malformed_is_error() {
    let r = pp("`timescale 1ns/1us\nmodule m; endmodule\n");
    assert!(r.has_errors(), "coarse precision must error");
}

#[test]
fn resolve_module_timescales_file_order_and_global_min() {
    let regions = [
        (
            10usize,
            TimeScale {
                unit_exp: -9,
                prec_exp: -10,
            },
        ), // 1ns/100ps
        (
            50usize,
            TimeScale {
                unit_exp: -8,
                prec_exp: -12,
            },
        ), // 10ns/1ps
    ];
    // a@20 → region@10 ; b@60 → region@50 ; c@5 (before any) → 1ns/1ns base.
    let modules = [("a", 20usize), ("b", 60usize), ("c", 5usize)];
    let r = resolve_module_timescales(&modules, &regions);
    assert_eq!(r.unit_exp["a"], -9);
    assert_eq!(r.unit_exp["b"], -8);
    assert_eq!(r.unit_exp["c"], -9); // default
    assert_eq!(r.global_prec_exp, -12); // min(-10, -12, -9)
    assert!(r.default_used); // c fell back
}

#[test]
fn resolve_no_regions_is_default() {
    let r = resolve_module_timescales(&[("m", 0)], &[]);
    assert_eq!(r.unit_exp["m"], -9);
    assert_eq!(r.global_prec_exp, -9);
    assert!(r.default_used);
}
/// `files` keys are paths RELATIVE to "/virtual" (joined for the shim map).
fn pp_mem(src: &str, files: &[(&str, &str)]) -> PpResult {
    let reader = MemReader {
        files: files
            .iter()
            .map(|(k, v)| {
                (
                    MemReader::norm(&Path::new("/virtual").join(k)),
                    v.to_string(),
                )
            })
            .collect(),
    };
    preprocess_with(
        Path::new("/virtual"),
        "top.sv",
        src,
        &PreOpts::default(),
        &reader,
    )
}
fn codes(r: &PpResult) -> Vec<&'static str> {
    r.diags.iter().map(|d| d.code.mnemonic()).collect()
}

// 1. object-like macro
#[test]
fn object_macro_expands() {
    let r = pp("`define W 8\nwire [`W-1:0] x;\n");
    assert!(r.diags.is_empty());
    assert_eq!(r.text, "\nwire [8-1:0] x;\n");
}

// 2. function-like macro
#[test]
fn function_macro_expands() {
    let r = pp("`define MAX(a,b) ((a)>(b)?(a):(b))\nassign y = `MAX(p, q);\n");
    assert!(r.diags.is_empty());
    assert_eq!(r.text, "\nassign y = ((p)>(q)?(p):(q));\n");
}

// DIR-PP: token-paste `` in a function-like macro body (IEEE 1800 §22.5.2).
#[test]
fn token_paste_function_macro() {
    let r = pp("`define CAT(a,b) a``b\n`CAT(foo,bar)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\nfoobar\n");
}

// DIR-PP: paste a prefix onto an argument to build an identifier.
#[test]
fn token_paste_prefix_ident() {
    let r = pp("`define REG(n) reg_``n\n`REG(count)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\nreg_count\n");
}

// DIR-PP: chained paste a``b``c.
#[test]
fn token_paste_chained() {
    let r = pp("`define D(a,b,c) a``b``c\n`D(x,y,z)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\nxyz\n");
}

// DIR-PP: paste deletes the two-backtick operator but PRESERVES adjacent
// whitespace (iverilog parity: `a `` b` => `a  b`, NOT `ab`).
#[test]
fn token_paste_preserves_surrounding_whitespace() {
    let r = pp("`define C(a,b) a `` b\n`C(foo,bar)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\nfoo  bar\n");
}

// DIR-PP: token-paste operator INSIDE a stringification is also deleted
// (whitespace preserved): `"a `` b`" => "a  b"; tight `"a``b`" => "xy".
#[test]
fn token_paste_inside_stringify() {
    let r = pp("`define J(a,b) `\"a `` b`\"\n`J(x,y)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\n\"x  y\"\n");
    let r2 = pp("`define J2(a,b) `\"a``b`\"\n`J2(x,y)\n");
    assert!(r2.diags.is_empty(), "{:?}", r2.diags);
    assert_eq!(r2.text, "\n\"xy\"\n");
}

// DIR-PP: object-like macro paste (no params) — routed through substitute too.
#[test]
fn token_paste_object_macro() {
    let r = pp("`define J x``y\n`J\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\nxy\n");
}

// DIR-PP: stringification `"x`" with argument substitution (IEEE 1800 §22.5.1).
#[test]
fn stringify_basic() {
    let r = pp("`define STR(x) `\"x`\"\n`STR(hi)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\n\"hi\"\n");
}

// DIR-PP: stringify preserves literal text + spaces around the arg.
#[test]
fn stringify_spaces_and_text() {
    let r = pp("`define P(a,b) `\"a and b`\"\n`P(x,y)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\n\"x and y\"\n");
}

// DIR-PP: `\`" inside a stringify produces an escaped quote (=> `\"`).
#[test]
fn stringify_embedded_escaped_quote() {
    let r = pp("`define Q(x) `\"a `\\`\"x`\\`\" b`\"\n`Q(z)\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    // expansion: "a \"z\" b"
    assert_eq!(r.text, "\n\"a \\\"z\\\" b\"\n");
}

// DIR-PP: object-like stringify.
#[test]
fn stringify_object_macro() {
    let r = pp("`define LIT `\"text`\"\n`LIT\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert_eq!(r.text, "\n\"text\"\n");
}

// DIR-PP: `begin_keywords/`end_keywords are accepted and stripped (no diag).
#[test]
fn begin_end_keywords_accepted() {
    let r = pp("`begin_keywords \"1364-2005\"\nmodule m; endmodule\n`end_keywords\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert!(r.text.contains("module m; endmodule"));
    assert!(!r.text.contains("begin_keywords"));
    assert!(!r.text.contains("end_keywords"));
}

// DIR-PP: `unconnected_drive/`nounconnected_drive accepted and stripped.
#[test]
fn unconnected_drive_accepted() {
    let r = pp("`unconnected_drive pull1\nmodule m; endmodule\n`nounconnected_drive\n");
    assert!(r.diags.is_empty(), "{:?}", r.diags);
    assert!(r.text.contains("module m; endmodule"));
    assert!(!r.text.contains("unconnected_drive"));
}

// 3. multi-line continuation in a macro body
#[test]
fn line_continuation_joins_body() {
    let r = pp("`define LONG aaa \\\nbbb\nx = `LONG;\n");
    assert!(r.diags.is_empty());
    assert_eq!(r.text, "x = aaa \nbbb;\n");
}

// 4. nested ifdef/ifndef/elsif/else/endif
#[test]
fn conditional_arms() {
    let src = "\
`define A
`ifdef A
keepA
`elsif B
dropB
`else
dropE
`endif
`ifndef A
dropN
`else
keepM
`endif
";
    let r = pp(src);
    assert!(r.diags.is_empty());
    assert!(r.text.contains("keepA"));
    assert!(r.text.contains("keepM"));
    assert!(!r.text.contains("dropB"));
    assert!(!r.text.contains("dropE"));
    assert!(!r.text.contains("dropN"));
}

// 5. undef removes a macro (later use becomes E-PP-BAD-DIRECTIVE)
#[test]
fn undef_removes_macro() {
    let r = pp("`define X 1\n`undef X\nv = `X;\n");
    assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
    assert!(r.text.contains("`X")); // emitted literally after undef
}

// 6. arity error
#[test]
fn arity_mismatch_errors() {
    let r = pp("`define F(a,b) (a+b)\nz = `F(1);\n");
    assert_eq!(codes(&r), vec!["E-PP-MACRO-ARITY"]);
}

// 7. recursive-macro guard terminates and reports
#[test]
fn recursive_macro_guarded() {
    let r = pp("`define R `R\nq = `R;\n");
    assert_eq!(codes(&r), vec!["E-PP-RECURSIVE-MACRO"]);
    assert!(r.text.contains("`R")); // left literal, finite output
}

// 8. comma inside parens does NOT split args
#[test]
fn comma_in_parens_not_split() {
    let r = pp("`define G(x) [x]\ny = `G(foo(a, b));\n");
    assert!(r.diags.is_empty(), "got {:?}", codes(&r));
    assert_eq!(r.text, "\ny = [foo(a, b)];\n");
}

// 9. comma inside a string does NOT split args
#[test]
fn comma_in_string_not_split() {
    let r = pp("`define S(x) {x}\ny = `S(\"a,b\");\n");
    assert!(r.diags.is_empty(), "got {:?}", codes(&r));
    assert_eq!(r.text, "\ny = {\"a,b\"};\n");
}

// 10. a macro-looking token inside a string is NOT expanded
#[test]
fn backtick_in_string_not_expanded() {
    let r = pp("`define M xyz\nv = \"`M\";\n");
    assert!(r.diags.is_empty());
    assert_eq!(r.text, "\nv = \"`M\";\n"); // string preserved verbatim
}

// 11. include happy path (in-memory shim), defines persist after include
#[test]
fn include_happy_path() {
    let r = pp_mem(
        "`include \"defs.svh\"\nwire [`WIDTH-1:0] bus;\n",
        &[("defs.svh", "`define WIDTH 16\n")],
    );
    assert!(r.diags.is_empty(), "got {:?}", codes(&r));
    assert!(r.text.contains("wire [16-1:0] bus;"));
}

// 12. include cycle guard
#[test]
fn include_cycle_guarded() {
    let r = pp_mem(
        "`include \"a.svh\"\n",
        &[
            ("a.svh", "`include \"b.svh\"\n"),
            ("b.svh", "`include \"a.svh\"\n"), // would re-open a.svh
        ],
    );
    assert!(codes(&r).contains(&"E-PP-RECURSIVE-INCLUDE"));
}

// 13. include not found
#[test]
fn include_not_found() {
    let r = pp_mem("`include \"missing.svh\"\n", &[]);
    assert_eq!(codes(&r), vec!["E-PP-INCLUDE-NOT-FOUND"]);
}

// 14. redefine warning (different body); identical redefine is silent
#[test]
fn redefine_warns_only_on_difference() {
    let r = pp("`define D 1\n`define D 1\n`define D 2\n");
    assert_eq!(codes(&r), vec!["W-PP-MACRO-REDEFINED"]); // exactly one (the 1->2)
}

// 15. undef of an undefined name warns
#[test]
fn undef_undefined_warns() {
    let r = pp("`undef NOPE\n");
    assert_eq!(codes(&r), vec!["W-PP-UNDEF-UNDEFINED"]);
}

// 16. unknown directive errors
#[test]
fn unknown_directive_errors() {
    let r = pp("`frobnicate foo\n");
    assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
}

// 17. SOURCE-MAP round trip
#[test]
fn source_map_round_trip() {
    let src = "`define P qq\n\nz = `P + bad;\n";
    let r = pp(src);
    assert!(r.diags.is_empty());
    let exp_off = r.text.find("bad").unwrap();
    let loc = r.map.resolve(exp_off);
    assert_eq!(loc.file_name, "top.sv");
    assert_eq!(
        loc.line, 3,
        "expanded offset must map back to original line 3"
    );
    let exp_qq = r.text.find("qq").unwrap();
    let loc_qq = r.map.resolve(exp_qq);
    assert_eq!(
        loc_qq.line, 3,
        "expanded macro text collapses to the use site"
    );
}

// 18. unterminated string reported as E-PP-BAD-DIRECTIVE
#[test]
fn unterminated_string_reported() {
    let r = pp("v = \"abc\nnext;\n");
    assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
}

// 19. SOURCE-MAP verbatim fidelity
#[test]
fn verbatim_region_resolves_byte_for_byte() {
    let src = "module m;\n  wire w;\nendmodule\n";
    let r = pp(src);
    assert!(r.diags.is_empty());
    assert_eq!(r.text, src); // identity fast path
    for off in [0usize, 10, 20, src.len()] {
        let loc = r.map.resolve(off);
        let (line, col) = byte_to_line_col(src, off);
        assert_eq!((loc.line, loc.col), (line, col), "verbatim off={off}");
        assert_eq!(loc.orig_byte as usize, off.min(src.len()));
    }
}

// 20. define significant-space rule
#[test]
fn define_significant_space_makes_object_like() {
    let r1 = pp("`define F (x)\ny = `F;\n");
    assert!(r1.diags.is_empty(), "got {:?}", codes(&r1));
    assert_eq!(r1.text, "\ny = (x);\n");
    let r2 = pp("`define G(x) (x+1)\ny = `G(1);\n");
    assert!(r2.diags.is_empty(), "got {:?}", codes(&r2));
    assert_eq!(r2.text, "\ny = (1+1);\n");
}

// 21. recursion guard scoping: `A in an arg to `A is a SIBLING use
#[test]
fn macro_name_in_argument_is_not_recursive() {
    let r = pp("`define B z\n`define A(x) [x]\ny = `A(`B);\n");
    assert!(r.diags.is_empty(), "got {:?}", codes(&r));
    assert_eq!(r.text, "\ny = [z];\n");
}

// 22. unterminated macro argument list is ALWAYS an error
#[test]
fn unterminated_macro_call_errors() {
    let r = pp("`define MAX(a,b) ((a)>(b)?(a):(b))\nz = `MAX(p, q\n");
    assert_eq!(codes(&r), vec!["E-PP-MACRO-ARITY"]);
}

// 23. two unclosed `ifdef`s report at two DISTINCT lines
#[test]
fn two_unclosed_ifdefs_report_distinct_lines() {
    let r = pp("`ifdef A\n`ifdef B\nx\n");
    let errs: Vec<_> = r
        .diags
        .iter()
        .filter(|d| d.code.mnemonic() == "E-PP-BAD-DIRECTIVE")
        .map(|d| r.map.resolve(d.at).line)
        .collect();
    assert_eq!(errs.len(), 2, "two unterminated frames");
    assert_ne!(errs[0], errs[1], "distinct opening lines, not both EOF");
    assert!(errs.contains(&1) && errs.contains(&2));
}

// 24. include path supplied via a macro
#[test]
fn include_path_via_macro() {
    let r = pp_mem(
        "`define INC \"f.svh\"\n`include `INC\nwire [`W-1:0] b;\n",
        &[("f.svh", "`define W 4\n")],
    );
    assert!(r.diags.is_empty(), "got {:?}", codes(&r));
    assert!(r.text.contains("wire [4-1:0] b;"));
}

// 25. NESTED include resolves relative to the INCLUDING file's own directory
#[test]
fn nested_include_uses_including_file_dir() {
    let r = pp_mem(
        "`include \"sub/b.svh\"\nwire [`N-1:0] z;\n",
        &[
            ("sub/b.svh", "`include \"c.svh\"\n"),
            ("sub/c.svh", "`define N 3\n"), // only in sub/, not at entry dir
        ],
    );
    assert!(r.diags.is_empty(), "got {:?}", codes(&r));
    assert!(r.text.contains("wire [3-1:0] z;"));
}

// 26. P2-12 policy: `` `pragma <rest-of-line> `` is accepted and ignored
// (IEEE 1800 §22.11) — previously misparsed as an undefined macro use.
#[test]
fn pragma_directive_accepted_and_ignored() {
    let r = pp_mem(
        "`pragma protect begin\nmodule m; endmodule\n`pragma translate_off // tail\n",
        &[],
    );
    assert!(!r.has_errors(), "diags: {:?}", r.diags);
    assert!(r.text.contains("module m"));
    assert!(!r.text.contains("pragma"));
}

// 27. byte_to_line_col / SourceMap::resolve never panic on a byte that lands
// mid-UTF-8-scalar (a resolved orig_byte can fall inside a multibyte char).
#[test]
fn resolve_mid_char_no_panic() {
    let src = "// 한글 주석\n`define W 8\nwire [`W-1:0] x;\n";
    // Every byte offset (incl. mid-scalar ones in the comment) must be safe.
    for b in 0..=src.len() + 4 {
        let (line, col) = byte_to_line_col(src, b);
        assert!(line >= 1 && col >= 1);
    }
    // And through the public SourceMap on the expanded text.
    let r = pp_mem(src, &[]);
    for b in 0..=r.text.len() + 4 {
        let _ = r.map.resolve(b);
    }
}

/// V33-8: the memoized line index and the linear walk are two spellings of ONE
/// answer, so a diagnostic's `line:col` must not depend on which one a emitter
/// reached for. Fuzzed over the shapes that break naive versions of this: CRLF,
/// a file that does not end in a newline, consecutive newlines, a leading
/// newline, multibyte scalars (so the offset can land mid-scalar and both must
/// floor it the same way), and every byte offset including past the end.
#[test]
fn line_col_index_matches_the_linear_walk() {
    let corpus = [
        "",
        "\n",
        "a",
        "a\n",
        "\na",
        "one\ntwo\nthree",
        "one\r\ntwo\r\n",
        "a\n\n\nb",
        "héllo\nwörld\n日本語",
        "x\n😀y\nz",
    ];
    for src in corpus {
        let starts = crate::line_starts_of(src);
        // `len() + 2` so the clamp path (an offset past EOF) is covered too.
        for b in 0..src.len() + 2 {
            assert_eq!(
                crate::byte_to_line_col(src, b),
                crate::byte_to_line_col_indexed(src, &starts, b),
                "src={src:?} byte={b}"
            );
        }
    }
}
