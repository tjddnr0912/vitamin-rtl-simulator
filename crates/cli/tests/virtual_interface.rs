//! N7-REST ⓑ-breadth: virtual interfaces (IEEE 1800 §25.9).
//!
//! iverilog 13 does NOT support virtual interfaces (syntax error), so the oracle is
//! hand-IEEE. vita models a `virtual IFACE vif;` as a STATIC ALIAS: when `vif` is
//! bound once to a concrete interface instance (`vif = bif;`), every `vif.signal`
//! access resolves to that instance's flattened net. Dynamic/conditional re-binding
//! is a v1 loud-reject (honest, never silent).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_vif_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, err, ok) = run_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn vif_read_through_binding() {
    // bind vif to bif, write bif.data directly, read it through vif.
    let out = run(
        "interface bus_if; logic [7:0] data; logic valid; endinterface\n\
         module t;\n\
           bus_if bif();\n\
           virtual bus_if vif;\n\
           initial begin\n\
             vif = bif;\n\
             bif.data = 8'hAB; bif.valid = 1;\n\
             $display(\"d=%0h v=%0b\", vif.data, vif.valid);\n\
           end\n\
         endmodule\n",
    );
    assert_eq!(out, "d=ab v=1\n");
}

#[test]
fn vif_write_through_binding() {
    // write THROUGH vif, read back on the bound instance.
    let out = run("interface bus_if; logic [7:0] data; endinterface\n\
         module t;\n\
           bus_if bif();\n\
           virtual bus_if vif;\n\
           initial begin\n\
             vif = bif;\n\
             vif.data = 8'hCD;\n\
             $display(\"d=%0h\", bif.data);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "d=cd\n");
}

fn assert_loud(src: &str, what: &str) {
    let (_, err, ok) = run_full(src);
    assert!(!ok, "{what}: must exit non-zero (loud)");
    assert!(
        err.contains("E3009") || err.contains("E3002") || err.contains("E30"),
        "{what}: expected an E-code; got:\n{err}"
    );
}

#[test]
fn unbound_vif_is_loud() {
    assert_loud(
        "interface bus_if; logic [7:0] data; endinterface\n\
         module t; bus_if bif(); virtual bus_if vif;\n\
         initial $display(\"%0h\", vif.data); endmodule\n",
        "unbound vif",
    );
}

#[test]
fn dynamic_rebind_is_loud() {
    assert_loud(
        "interface bus_if; logic [7:0] data; endinterface\n\
         module t; bus_if a(); bus_if b(); virtual bus_if vif;\n\
         initial begin vif = a; vif = b; vif.data = 1; end endmodule\n",
        "dynamic rebind",
    );
}

#[test]
fn wrong_iface_type_is_loud() {
    assert_loud(
        "interface a_if; logic x; endinterface\n\
         interface b_if; logic y; endinterface\n\
         module t; b_if bi(); virtual a_if vif;\n\
         initial begin vif = bi; vif.x = 1; end endmodule\n",
        "wrong iface type",
    );
}
