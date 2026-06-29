//! v5 ⑥ (D): SystemVerilog interface front-end — flattening to plain nets +
//! symbol aliasing (spike 2026-06-10: SimIr untouched, `.vu` flip only).
//!
//! iverilog 13.0 rejects interface-typed module ports outright (probed live
//! 2026-06-11: "Errors in port declarations"), so like the assoc lanes these
//! pins are HAND-IEEE (1800 §25): an interface instance is a bundle of nets;
//! a module's interface port makes the bundle's members visible as
//! `<port>.<member>` inside the module, referring to the SAME nets.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_vita_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_iface_{}_{n}.sv", std::process::id()));
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

fn run_vita(src: &str) -> String {
    let (out, err, ok) = run_vita_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn assert_loud(src: &str, what: &str) {
    let (_, err, ok) = run_vita_full(src);
    assert!(!ok, "{what}: must exit non-zero (loud)");
    assert!(
        err.contains("VITA-E"),
        "{what}: E-code expected; got:\n{err}"
    );
}

#[test]
fn iface_port_aliases_members() {
    // The child reads THROUGH the port; the parent drives the instance —
    // same nets, no copies (hand-IEEE §25.3).
    let src = r#"
interface bus_if;
  logic [7:0] data;
  logic valid;
endinterface

module sink(bus_if b);
  initial begin
    #1;
    $display("%0d %0d", b.data, b.valid);
  end
endmodule

module t;
  bus_if i();
  sink u(.b(i));
  initial begin
    i.data = 8'd42;
    i.valid = 1'b1;
  end
endmodule
"#;
    assert_eq!(run_vita(src), "42 1\n");
}

#[test]
fn iface_internal_logic_and_proc_run() {
    // Cont-assigns AND procs INSIDE the interface lower like a module body
    // (ModuleDecl reuse) — parity is computed in the interface itself.
    let src = r#"
interface par_if;
  logic [3:0] d;
  logic p;
  assign p = ^d;
  initial $display("iface-proc");
endinterface

module t;
  par_if i();
  initial begin
    i.d = 4'b1011;
    #1;
    $display("%0d", i.p);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "iface-proc\n1\n");
}

#[test]
fn iface_modport_port_type_accepted() {
    // `bus_if.consumer b` — the modport-qualified port type parses and binds
    // (direction ENFORCEMENT is a documented follow-on; existence is checked).
    let src = r#"
interface bus_if;
  logic [7:0] data;
  modport consumer (input data);
endinterface

module sink(bus_if.consumer b);
  initial begin
    #1;
    $display("%0d", b.data);
  end
endmodule

module t;
  bus_if i();
  sink u(.b(i));
  initial i.data = 8'd7;
endmodule
"#;
    assert_eq!(run_vita(src), "7\n");
}

#[test]
fn iface_edge_sensitivity_through_alias() {
    // @(posedge b.clk) inside the child fires on the parent's toggle of
    // i.clk — the alias IS the same net, so the dirty channel just works.
    let src = r#"
interface clk_if;
  logic clk;
endinterface

module watcher(clk_if b);
  always @(posedge b.clk) $display("rose");
endmodule

module t;
  clk_if i();
  watcher u(.b(i));
  initial begin
    i.clk = 0;
    #1 i.clk = 1;
    #1 $finish;
  end
endmodule
"#;
    assert_eq!(run_vita(src), "rose\n");
}

#[test]
fn iface_two_instances_are_independent() {
    let src = r#"
interface v_if;
  logic [7:0] x;
endinterface

module t;
  v_if a();
  v_if b();
  initial begin
    a.x = 8'd1;
    b.x = 8'd2;
    $display("%0d %0d", a.x, b.x);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "1 2\n");
}

// ───────────────────────── loud lanes ─────────────────────────

#[test]
fn iface_mismatches_are_loud() {
    // connecting a plain net to an interface port
    assert_loud(
        r#"
interface bus_if;
  logic d;
endinterface
module sink(bus_if b);
endmodule
module t;
  wire w;
  sink u(.b(w));
endmodule
"#,
        "non-instance connection",
    );
    // interface TYPE mismatch
    assert_loud(
        r#"
interface a_if;
  logic d;
endinterface
interface b_if;
  logic d;
endinterface
module sink(a_if b);
endmodule
module t;
  b_if i();
  sink u(.b(i));
endmodule
"#,
        "interface type mismatch",
    );
    // unknown modport name
    assert_loud(
        r#"
interface bus_if;
  logic d;
  modport mp (input d);
endinterface
module sink(bus_if.nope b);
endmodule
module t;
  bus_if i();
  sink u(.b(i));
endmodule
"#,
        "unknown modport",
    );
    // unconnected interface port
    assert_loud(
        r#"
interface bus_if;
  logic d;
endinterface
module sink(bus_if b);
endmodule
module t;
  sink u();
endmodule
"#,
        "unconnected interface port",
    );
}

#[test]
fn iface_mvp_cuts_are_loud() {
    // nested module instance inside an interface
    assert_loud(
        r#"
module leaf;
endmodule
interface bus_if;
  leaf l();
endinterface
module t;
  bus_if i();
endmodule
"#,
        "nested instance in interface",
    );
}

#[test]
fn duplicate_unit_name_is_loud() {
    assert_loud(
        r#"
interface x;
  logic d;
endinterface
module x;
endmodule
module t;
  initial $display("hi");
endmodule
"#,
        "module/interface name collision",
    );
}

// ───────────────────────── ② interface follow-ons (v6 batch) ─────────────────────────

#[test]
fn modport_input_write_is_loud() {
    // hand-IEEE §25.5: a modport `input` member is read-only inside the
    // module bound through that modport — a write is an error.
    let src = r#"
interface bus_if;
  logic req;
  logic ack;
  modport consumer (input req, output ack);
endinterface

module sink(bus_if.consumer p);
  initial p.req = 1'b1;
endmodule

module t;
  bus_if i();
  sink u(.p(i));
endmodule
"#;
    assert_loud(src, "write through a modport input");
}

#[test]
fn modport_input_nba_and_cont_assign_writes_are_loud() {
    let nba = r#"
interface bus_if;
  logic req;
  modport consumer (input req);
endinterface
module sink(bus_if.consumer p);
  initial p.req <= 1'b1;
endmodule
module t;
  bus_if i();
  sink u(.p(i));
endmodule
"#;
    assert_loud(nba, "NBA write through a modport input");
    let ca = r#"
interface bus_if;
  logic req;
  modport consumer (input req);
endinterface
module sink(bus_if.consumer p);
  assign p.req = 1'b0;
endmodule
module t;
  bus_if i();
  sink u(.p(i));
endmodule
"#;
    assert_loud(ca, "cont-assign through a modport input");
}

#[test]
fn modport_output_write_reaches_parent() {
    // Output members stay writable; the write lands on the SAME net the
    // parent sees (aliasing, not a copy).
    let src = r#"
interface bus_if;
  logic req;
  logic ack;
  modport consumer (input req, output ack);
endinterface

module sink(bus_if.consumer p);
  initial begin
    #1;
    if (p.req) p.ack = 1'b1;
  end
endmodule

module t;
  bus_if i();
  sink u(.p(i));
  initial i.req = 1'b1;
  initial begin
    #2;
    $display("ack=%0d", i.ack);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "ack=1\n");
}

#[test]
fn modport_unlisted_member_is_invisible() {
    // §25.5: a modport RESTRICTS access to its listed members — an unlisted
    // member is not visible through the port at all.
    let src = r#"
interface bus_if;
  logic req;
  logic secret;
  modport consumer (input req);
endinterface

module sink(bus_if.consumer p);
  initial $display("%0d", p.secret);
endmodule

module t;
  bus_if i();
  sink u(.p(i));
endmodule
"#;
    assert_loud(src, "unlisted modport member");
}

#[test]
fn iface_parameter_override() {
    // interface #(parameter W) — per-instance override widens the bundle.
    let src = r#"
interface bus_if #(parameter W = 4);
  logic [W-1:0] d;
endinterface

module t;
  bus_if #(8) wide();
  bus_if narrow();
  initial begin
    wide.d = 8'd165;
    narrow.d = 8'd165; // truncates to 4 bits → 5
    #1;
    $display("%0d %0d", wide.d, narrow.d);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "165 5\n");
}

#[test]
fn iface_header_input_port_wires_parent_expr() {
    // interface HEADER port (ANSI input): the parent connection drives the
    // iface-internal port net, visible to the iface body and via i.<port>.
    let src = r#"
interface bus_if(input logic c);
  logic d;
  assign d = c;
endinterface

module t;
  logic clk;
  bus_if b(clk);
  initial begin
    clk = 1'b1;
    #1;
    $display("%0d %0d", b.c, b.d);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "1 1\n");
}

#[test]
fn iface_header_output_port_drives_parent() {
    let src = r#"
interface bus_if(output logic ready);
  initial ready = 1'b1;
endinterface

module t;
  logic r;
  bus_if b(r);
  initial begin
    #1;
    $display("%0d", r);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "1\n");
}

#[test]
fn generate_nested_child_takes_iface_port() {
    // A child instantiated INSIDE a generate block binds an iface declared at
    // the top level — the binding lookup walks generate scopes outward.
    let src = r#"
interface bus_if;
  logic [7:0] d;
endinterface

module sink(bus_if b);
  initial begin
    #1;
    $display("%0d", b.d);
  end
endmodule

module t;
  bus_if i();
  genvar g;
  generate
    for (g = 0; g < 1; g = g + 1) begin : blk
      sink u(.b(i));
    end
  endgenerate
  initial i.d = 8'd77;
endmodule
"#;
    assert_eq!(run_vita(src), "77\n");
}

#[test]
fn iface_header_nonansi_ports_stay_loud() {
    let src = r#"
interface bus_if(c);
  input c;
endinterface

module t;
  logic clk;
  bus_if b(clk);
endmodule
"#;
    assert_loud(src, "non-ANSI interface header ports");
}
