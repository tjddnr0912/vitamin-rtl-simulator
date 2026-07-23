//! r18 (E2): a method call whose receiver is a struct MEMBER (`r.name.substr(8,15)`,
//! where `r.name` is a `string` member) is now supported — was E3009 "unsupported
//! hierarchical function call `r.name.substr`", because the 3-segment path `r.name.substr`
//! parsed as one hierarchical Call. The parser now rewrites the receiver `r.name` to its
//! member net `$unp$r$name`, so the call becomes the 2-segment `$unp$r$name.substr(a,b)`
//! form elaborate already dispatches as a string method.
//!
//! ORACLE: iverilog 13.0 rejects unpacked structs entirely, so there is no direct oracle;
//! these are hand-IEEE, cross-checked against the SAME string method on a bare string
//! variable (which vita+iverilog both run identically).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_smm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009") || o.contains("E2002")
}

// ── the report's E2: `.substr(a,b)` on a struct string-member ──
#[test]
fn substr_on_string_member() {
    let o = run("module t;\n\
        typedef struct { string name; } rec_t;\n\
        initial begin\n\
          rec_t r; r.name = \"prefix--1MB-of-a\";\n\
          if (r.name.substr(8,15) == \"1MB-of-a\") $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("PASS"), "E2 repro:\n{o}");
}

// ── `.len()` / `.toupper()` / `.tolower()` on a struct string-member ──
#[test]
fn len_and_case_methods_on_member() {
    let o = run("module t;\n\
        typedef struct { string name; int id; } rec_t;\n\
        initial begin\n\
          rec_t r; r.name = \"AbC\"; r.id = 7;\n\
          $display(\"len=%0d up=%s lo=%s\", r.name.len(), r.name.toupper(), r.name.tolower());\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("len=3 up=ABC lo=abc"),
        "member len/toupper/tolower:\n{o}"
    );
}

// ── a method CHAINED on a struct-member string method (`.substr().atoi()`) ──
#[test]
fn chained_method_on_member_result() {
    let o = run("module t;\n\
        typedef struct { string name; } rec_t;\n\
        initial begin\n\
          rec_t r; r.name = \"prefix--42\";\n\
          if (r.name.substr(8,9).atoi() == 42) $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "chained on member:\n{o}"
    );
}

// ── regression: a bare string-var method still works (unchanged path) ──
#[test]
fn bare_string_var_method_unchanged() {
    let o = run("module t;\n\
        initial begin\n\
          string s; s = \"prefix--1MB-of-a\";\n\
          if (s.substr(8,15) == \"1MB-of-a\") $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "bare string method:\n{o}"
    );
}

// ── a bare struct string-member READ still works (unchanged) ──
#[test]
fn bare_member_read_unchanged() {
    let o = run("module t;\n\
        typedef struct { string name; } rec_t;\n\
        initial begin\n\
          rec_t r; r.name = \"hello\";\n\
          $display(\"n=%s\", r.name);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("n=hello"), "member read:\n{o}");
}
