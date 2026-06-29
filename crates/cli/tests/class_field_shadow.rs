//! A derived class field that redeclares an inherited base field name is
//! SHADOWING (IEEE §8.14): the base and derived fields get DISTINCT storage. This
//! was loud-rejected ("distinct base/derived storage is unsupported"); now both
//! slots are kept and a bare/`this` field reference resolves to the most-derived
//! field visible in the referencing method's class (a base method sees the base
//! field; a derived method and an external `obj.f` see the derived field).
//! Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_shadow_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn distinct_base_and_derived_storage() {
    let out = run("class Base;\n\
           int v;\n\
           function int get_base(); return v; endfunction\n\
           function void set_base(int x); v = x; endfunction\n\
         endclass\n\
         class Derived extends Base;\n\
           int v;\n\
           function int get_der(); return v; endfunction\n\
         endclass\n\
         module t;\n\
           Derived d;\n\
           initial begin\n\
             d = new;\n\
             d.set_base(11);\n\
             d.v = 22;\n\
             $display(\"der=%0d base=%0d\", d.get_der(), d.get_base());\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "der=22 base=11\n");
}

#[test]
fn external_access_is_most_derived() {
    // `d.v` (external, static type Derived) reaches Derived::v, leaving Base::v.
    let out = run("class Base; int v; endclass\n\
         class Derived extends Base; int v; endclass\n\
         module t;\n\
           Derived d;\n\
           initial begin d = new; d.v = 9; $display(\"%0d\", d.v); end\n\
         endmodule\n");
    assert_eq!(out, "9\n");
}

#[test]
fn no_shadow_still_works() {
    // CONTROL: a derived field with a DISTINCT name keeps the flat single-slot
    // layout (byte-identical) and inherits the base field normally.
    let out = run("class Base; int a; endclass\n\
         class Derived extends Base; int b; endclass\n\
         module t;\n\
           Derived d;\n\
           initial begin d = new; d.a = 3; d.b = 4; $display(\"%0d %0d\", d.a, d.b); end\n\
         endmodule\n");
    assert_eq!(out, "3 4\n");
}
