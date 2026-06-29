//! DIR-PP (Medium bundle, rank 1) — preprocessor leftovers, end-to-end through vita.
//!
//! Token-paste (` `` `), stringification (` `" `), and the `begin_keywords`/
//! `end_keywords` + `unconnected_drive`/`nounconnected_drive` directives were
//! previously broken-loud (stray-backtick / undefined-directive errors). These run a
//! real design that uses each and check the simulated output. iverilog 13.0 is the
//! differential oracle (each expected value verified live: `v=5 s=hello`, `ok=1`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dirpp_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn token_paste_builds_identifier_and_stringify_renders() {
    // `CAT(my,var) => identifier `myvar`; `STR(hello) => "hello".
    let out = run("`define CAT(a,b) a``b\n\
         `define STR(x) `\"x`\"\n\
         module m;\n\
           integer `CAT(my,var);\n\
           initial begin\n\
             `CAT(my,var) = 5;\n\
             $display(\"v=%0d s=%s\", `CAT(my,var), `STR(hello));\n\
           end\n\
         endmodule\n");
    assert!(out.contains("v=5 s=hello"), "paste+stringify e2e:\n{out}");
}

#[test]
fn stringify_with_embedded_escaped_quote() {
    // `Q(z) => "a \"z\" b" — the `\`" escape becomes a literal escaped quote.
    let out = run("`define Q(x) `\"a `\\`\"x`\\`\" b`\"\n\
         module m; initial $display(\"s=%s\", `Q(z)); endmodule\n");
    assert!(
        out.contains("s=a \"z\" b"),
        "stringify escaped quote:\n{out}"
    );
}

#[test]
fn begin_end_keywords_region_compiles_and_runs() {
    let out = run("`begin_keywords \"1364-2005\"\n\
         module m; initial $display(\"ok=%0d\", 1); endmodule\n\
         `end_keywords\n");
    assert!(out.contains("ok=1"), "begin_keywords region:\n{out}");
}

#[test]
fn unconnected_drive_directives_accepted() {
    let out = run("`unconnected_drive pull1\n\
         module m; initial $display(\"ud=%0d\", 7); endmodule\n\
         `nounconnected_drive\n");
    assert!(out.contains("ud=7"), "unconnected_drive accept:\n{out}");
}
