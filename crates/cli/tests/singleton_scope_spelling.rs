//! A SINGLETON generate scope (`generate if` / `case` / a bare labelled block) is
//! stored as `label[0]` and must PRINT as `label` everywhere a hierarchical name
//! reaches the user. `%m` and the `[in …]` context already did; the per-net name
//! table (`net_name_table`) did not, so one runtime diagnostic carried two spellings
//! of one scope on one line — `` `t.g_sw[0].tbl` `` beside `[in t.g_sw.g_l[0]]`
//! (reviewer §3.2, 2026-09-09) — and the VCD `$scope` for the block was `g_sw[0]`
//! where both reference tools write `g_sw`. Loop iterations keep their index.

use std::process::Command;

fn run(name: &str, body: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("vita_sss_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("d.sv");
    std::fs::write(&sv, body).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .current_dir(&dir)
        .args(["--top", "t"])
        .arg(&sv)
        .output()
        .expect("run vita");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    let vcd = std::fs::read_to_string(dir.join("d.vcd")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (text, vcd)
}

const DESIGN: &str = r#"
module t;
  logic [1:0] ix; logic [7:0] o;
  generate if (1) begin : g_sw
    logic [7:0] tbl [4] = '{1,2,3,4};
    for (genvar i = 0; i < 1; i++) begin : g_l
      always_comb o = tbl[ix];
    end
  end endgenerate
  initial begin $dumpfile("d.vcd"); $dumpvars; #1 ix = 1; #1 $display("%h", o); $finish; end
endmodule
"#;

#[test]
fn runtime_diagnostic_names_a_singleton_scope_the_way_its_context_does() {
    let (text, _) = run("diag", DESIGN);
    let line = text
        .lines()
        .find(|l| l.contains("VITA-W4029"))
        .unwrap_or_else(|| panic!("no W4029 line in:\n{text}"));
    assert!(line.contains("`t.g_sw.tbl`"), "{line}");
    assert!(!line.contains("g_sw[0]"), "{line}");
    // The loop scope keeps its index in the same line's context.
    assert!(line.contains("[in t.g_sw.g_l[0]]"), "{line}");
    assert!(text.starts_with("02\n"), "{text}");
}

#[test]
fn vcd_scope_of_a_singleton_generate_block_has_no_index() {
    let (_, vcd) = run("vcd", DESIGN);
    assert!(vcd.contains("$scope module g_sw $end"), "{vcd}");
    assert!(!vcd.contains("g_sw[0]"), "{vcd}");
}
