//! A REAL stored into a string converts by ONE rule on every store and every backend: §6.12.2
//! to a 64-bit signed integer (rounded), then §6.16's bytes — `Value::string_store_bytes`.
//! ROADMAP §2 "Real" (the string-variable-from-real bullet), closed by §4.5.549.
//!
//! Five stores spelled the conversion and only the container-element one (§4.5.540) converted a
//! real: a whole `string` variable took the IEEE-754 word on native (`s = 65.4` was
//! `405059999999999a`) and a one-bit value on the engine (`01`); a string function's return, a
//! task's `output string` and a string formal bound to a real took the IEEE word on every backend
//! (`40d05080` for 16706.0), except a STATIC task's formal on the engine and the VM (`00`).
//!
//! No oracle: iverilog aborts on every shape here, and verilator 5.052 converts the same value
//! three ways — a queue element by this rule ("AB" for 16706.0), a string formal as the raw
//! IEEE word, a whole variable as one low byte ("B") or an internal compiler error. It agrees with
//! this rule on every single-byte value (65.4 "A", 97.2 "a", 66.0 "B"). Each cell asserts the
//! native value and that the interpreter and the VM print the same lines.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, backend: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rstr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "expected exit 0 on {backend}, got:\n{s}"
    );
    s.lines()
        .filter(|l| l.starts_with('T'))
        .map(|l| l.trim().to_string())
        .collect()
}

fn check(src: &str, want: &[&str]) {
    let native = run(src, "native");
    assert_eq!(native, want, "native");
    assert_eq!(run(src, "interp"), native, "interp");
    assert_eq!(run(src, "vm"), native, "vm");
}

/// `s = <real>` by blocking, nonblocking and declaration-initializer stores, from a literal, a real variable and a real expression: 65.4 → "A", 16706.0 → "AB", −1.5 → the eight bytes of −2, 0.4 → "", 4294967393.0 → `01 61`.
#[test]
fn a_whole_string_variable() {
    check(
        r#"module t;
  string s;
  initial begin
    s = 65.4;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=41"#],
    );
    check(
        r#"module t;
  string s;
  initial begin
    s = 16706.0;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
    check(
        r#"module t;
  string s;
  initial begin
    s = -1.5;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=8 hex=fffffffffffffffe"#],
    );
    check(
        r#"module t;
  string s;
  initial begin
    s = 0.4;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=0 hex=00"#],
    );
    check(
        r#"module t;
  string s;
  initial begin
    s <= 97.2; #0;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=61"#],
    );
    check(
        r#"module t;
  string s = 97.2;
  initial begin
    
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=61"#],
    );
    check(
        r#"module t;
  string s; real r = 16706.0;
  initial begin
    s = r;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
    check(
        r#"module t;
  string s;
  initial begin
    s = 4294967393.0;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=0161"#],
    );
    check(
        r#"module t;
  string s; real r = 65.6;
  initial begin
    s = r * 1.0;
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=42"#],
    );
}

/// Automatic and static string functions (`f = 66.0`, `return x`, through a string local), a task's `output string`: every one was the IEEE word on every backend.
#[test]
fn a_string_function_return_a_task_output_and_a_string_formal() {
    check(
        r#"module t;
  string s; function automatic string f(); f = 66.0; endfunction
  initial begin
    s = f();
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=42"#],
    );
    check(
        r#"module t;
  string s;
  function automatic string f(); f = 66.0; endfunction
  initial begin
    s = f();
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=42"#],
    );
    check(
        r#"module t;
  string s;
  function string f(); f = 66.0; endfunction
  initial begin
    s = f();
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=1 hex=42"#],
    );
    check(
        r#"module t;
  string s;
  function automatic string f(input real x); f = x; endfunction
  initial begin
    s = f(16706.0);
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
    check(
        r#"module t;
  string s;
  function automatic string f(input real x); return x; endfunction
  initial begin
    s = f(16706.0);
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
    check(
        r#"module t;
  string s;
  function string f(input real x); return x; endfunction
  initial begin
    s = f(16706.0);
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
    check(
        r#"module t;
  string s;
  function automatic string f(input real x); string t; t = x; return t; endfunction
  initial begin
    s = f(16706.0);
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
    check(
        r#"module t;
  string s;
  task automatic tk(output string o, input real x); o = x; endtask
  initial begin
    tk(s, 16706.0);
    #1 $display("T len=%0d hex=%h", s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T len=2 hex=4142"#],
    );
}

/// §4.5.540's element lane beside the whole variable: the two agree.
#[test]
fn the_container_element_rule_is_the_same_rule() {
    check(
        r#"module t;
  string sq[$];
  string s;
  initial begin
    sq.push_back(16706.0);
    sq.push_back(4294967393.0);
    sq.push_back(65.4);
    sq.push_back(-1.5);
    s = 16706.0;
    #1 $display("T %0d %h | %0d %h | %0d %h | %0d %h | whole %0d %h", sq[0].len(), sq[0], sq[1].len(), sq[1], sq[2].len(), sq[2], sq[3].len(), sq[3], s.len(), s);
    $finish;
  end
endmodule
"#,
        &[r#"T 2 4142 | 2 0161 | 1 41 | 8 fffffffffffffffe | whole 2 4142"#],
    );
}
