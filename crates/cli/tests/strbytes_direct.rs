//! Q-STRBYTES pins, GENERATED from PLAN2's direct-form cells d1_val / d1_str / d1_sev / d1_lic (W/qstrbytes/plan2/d1) by W/qstrbytes/r1/bin/gen_pins.py.
//!
//! RTL text is bytes (IEEE 1800-2017 §21.2.1.8: a string is 8 bits per character, a field
//! width counts them). Each `_moves` test holds the lines this row moves (wrong -> right),
//! each `_stays` test the lines it must not move; every tagged line carries its source:
//! the oracle set whose output it equals (iv = iverilog 13, s2v = sv2v -> iverilog, vl =
//! verilator 5.052; their text quoted raw-escaped beside it), HAND (an IEEE value where no
//! oracle prints the line), JUDGED (a ruling against an oracle self-contradiction) or PRE
//! (a named stay). The comparison is raw bytes on every lane (`strbytes_util`).

mod strbytes_util;
use strbytes_util::{check, Cell, Mode};

const D_D1_VAL_SRC: &str = r#"module top;
  string t_ab = "ab", t_hi = "\310\377a", s_u8 = "\303\210a", s_e3 = "\342\202\254", u;
  logic [7:0] b_a = 8'h41;
  function automatic string fb_w3s(); string v; v = $sformatf("%3s", t_hi); return v; endfunction
  function automatic string fr_w3s(); return $sformatf("%3s", t_hi); endfunction
  function automatic string fb_w1s(); string v; v = $sformatf("%1s", t_hi); return v; endfunction
  function automatic string fr_w1s(); return $sformatf("%1s", t_hi); endfunction
  function automatic string fb_m3s(); string v; v = $sformatf("%-3s", t_hi); return v; endfunction
  function automatic string fr_m3s(); return $sformatf("%-3s", t_hi); endfunction
  function automatic string fb_z3s(); string v; v = $sformatf("%03s", t_hi); return v; endfunction
  function automatic string fr_z3s(); return $sformatf("%03s", t_hi); endfunction
  function automatic string fb_w3S(); string v; v = $sformatf("%3S", t_hi); return v; endfunction
  function automatic string fr_w3S(); return $sformatf("%3S", t_hi); endfunction
  function automatic string fb_sc(); string v; v = $sformatf("%s%c", t_hi, b_a); return v; endfunction
  function automatic string fr_sc(); return $sformatf("%s%c", t_hi, b_a); endfunction
  function automatic string fb_s3s(); string v; v = $sformatf("%s%3s", t_ab, t_hi); return v; endfunction
  function automatic string fr_s3s(); return $sformatf("%s%3s", t_ab, t_hi); endfunction
  function automatic string fb_u3s(); string v; v = $sformatf("%3s", s_u8); return v; endfunction
  function automatic string fr_u3s(); return $sformatf("%3s", s_u8); endfunction
  function automatic string fb_u4s(); string v; v = $sformatf("%4s", s_u8); return v; endfunction
  function automatic string fr_u4s(); return $sformatf("%4s", s_u8); endfunction
  function automatic string fb_um4(); string v; v = $sformatf("%-4s", s_u8); return v; endfunction
  function automatic string fr_um4(); return $sformatf("%-4s", s_u8); endfunction
  function automatic string fb_e3s(); string v; v = $sformatf("%3s", s_e3); return v; endfunction
  function automatic string fr_e3s(); return $sformatf("%3s", s_e3); endfunction
  function automatic string fb_e5s(); string v; v = $sformatf("%5s", s_e3); return v; endfunction
  function automatic string fr_e5s(); return $sformatf("%5s", s_e3); endfunction
  function automatic string fb_p0s(); string v; v = $sformatf("%0s", t_hi); return v; endfunction
  function automatic string fr_p0s(); return $sformatf("%0s", t_hi); endfunction
  function automatic string fb_pS(); string v; v = $sformatf("%S", t_hi); return v; endfunction
  function automatic string fr_pS(); return $sformatf("%S", t_hi); endfunction
  function automatic string fb_pms(); string v; v = $sformatf("%-s", t_hi); return v; endfunction
  function automatic string fr_pms(); return $sformatf("%-s", t_hi); endfunction
  function automatic string fb_ps(); string v; v = $sformatf("%s", t_hi); return v; endfunction
  function automatic string fr_ps(); return $sformatf("%s", t_hi); endfunction
  function automatic string fb_a5s(); string v; v = $sformatf("%5s", t_ab); return v; endfunction
  function automatic string fr_a5s(); return $sformatf("%5s", t_ab); endfunction
  function automatic string fb_a2s(); string v; v = $sformatf("%2s", t_ab); return v; endfunction
  function automatic string fr_a2s(); return $sformatf("%2s", t_ab); endfunction
  function automatic string fb_cat(); string v; v = {$sformatf("%3s", t_hi), "!"}; return v; endfunction
  task automatic show(input string tag); $display("%s %0d %h %h %h %h %h %h", tag, u.len(), u[0], u[1], u[2], u[3], u[4], u[5]); endtask
  initial begin
    u = $sformatf("%3s", t_hi); show("mod_w3s");
    u = $sformatf("%1s", t_hi); show("mod_w1s");
    u = $sformatf("%-3s", t_hi); show("mod_m3s");
    u = $sformatf("%03s", t_hi); show("mod_z3s");
    u = $sformatf("%3S", t_hi); show("mod_w3S");
    u = $sformatf("%s%c", t_hi, b_a); show("mod_sc");
    u = $sformatf("%s%3s", t_ab, t_hi); show("mod_s3s");
    u = $sformatf("%3s", s_u8); show("mod_u3s");
    u = $sformatf("%4s", s_u8); show("mod_u4s");
    u = $sformatf("%-4s", s_u8); show("mod_um4");
    u = $sformatf("%3s", s_e3); show("mod_e3s");
    u = $sformatf("%5s", s_e3); show("mod_e5s");
    u = $sformatf("%0s", t_hi); show("mod_p0s");
    u = $sformatf("%S", t_hi); show("mod_pS");
    u = $sformatf("%-s", t_hi); show("mod_pms");
    u = $sformatf("%s", t_hi); show("mod_ps");
    u = $sformatf("%5s", t_ab); show("mod_a5s");
    u = $sformatf("%2s", t_ab); show("mod_a2s");
    u = {$sformatf("%3s", t_hi), "!"}; show("mod_cat");
    u = $sformatf("%3s", t_hi); u = $sformatf("%s", u); show("mod_nst2");
    u = fb_w3s(); show("fnb_w3s");
    u = fb_w1s(); show("fnb_w1s");
    u = fb_m3s(); show("fnb_m3s");
    u = fb_z3s(); show("fnb_z3s");
    u = fb_w3S(); show("fnb_w3S");
    u = fb_sc(); show("fnb_sc");
    u = fb_s3s(); show("fnb_s3s");
    u = fb_u3s(); show("fnb_u3s");
    u = fb_u4s(); show("fnb_u4s");
    u = fb_um4(); show("fnb_um4");
    u = fb_e3s(); show("fnb_e3s");
    u = fb_e5s(); show("fnb_e5s");
    u = fb_p0s(); show("fnb_p0s");
    u = fb_pS(); show("fnb_pS");
    u = fb_pms(); show("fnb_pms");
    u = fb_ps(); show("fnb_ps");
    u = fb_a5s(); show("fnb_a5s");
    u = fb_a2s(); show("fnb_a2s");
    u = fb_cat(); show("fnb_cat");
    u = fr_w3s(); show("ret_w3s");
    u = fr_w1s(); show("ret_w1s");
    u = fr_m3s(); show("ret_m3s");
    u = fr_z3s(); show("ret_z3s");
    u = fr_w3S(); show("ret_w3S");
    u = fr_sc(); show("ret_sc");
    u = fr_s3s(); show("ret_s3s");
    u = fr_u3s(); show("ret_u3s");
    u = fr_u4s(); show("ret_u4s");
    u = fr_um4(); show("ret_um4");
    u = fr_e3s(); show("ret_e3s");
    u = fr_e5s(); show("ret_e5s");
    u = fr_p0s(); show("ret_p0s");
    u = fr_pS(); show("ret_pS");
    u = fr_pms(); show("ret_pms");
    u = fr_ps(); show("ret_ps");
    u = fr_a5s(); show("ret_a5s");
    u = fr_a2s(); show("ret_a2s");
    $sformat(u, "%3s", t_hi); show("sfm_w3s");
    $sformat(u, "%1s", t_hi); show("sfm_w1s");
    $sformat(u, "%-3s", t_hi); show("sfm_m3s");
    $sformat(u, "%03s", t_hi); show("sfm_z3s");
    $sformat(u, "%3S", t_hi); show("sfm_w3S");
    $sformat(u, "%s%c", t_hi, b_a); show("sfm_sc");
    $sformat(u, "%s%3s", t_ab, t_hi); show("sfm_s3s");
    $sformat(u, "%3s", s_u8); show("sfm_u3s");
    $sformat(u, "%4s", s_u8); show("sfm_u4s");
    $sformat(u, "%-4s", s_u8); show("sfm_um4");
    $sformat(u, "%3s", s_e3); show("sfm_e3s");
    $sformat(u, "%5s", s_e3); show("sfm_e5s");
    $sformat(u, "%0s", t_hi); show("sfm_p0s");
    $sformat(u, "%S", t_hi); show("sfm_pS");
    $sformat(u, "%-s", t_hi); show("sfm_pms");
    $sformat(u, "%s", t_hi); show("sfm_ps");
    $sformat(u, "%5s", t_ab); show("sfm_a5s");
    $sformat(u, "%2s", t_ab); show("sfm_a2s");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn d_d1_val_moves() {
    check(
        &Cell {
            src: D_D1_VAL_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // mod_w3s: MOVE = iv+s2v+vl ; PRE mod_w3s 7 ef bf bd ef bf bd
            //   iv: mod_w3s 3 c8 ff 61 00 00 00
            //   s2v: mod_w3s 3 c8 ff 61 00 00 00
            //   vl: mod_w3s 3 c8 ff 61 00 00 00
            ("mod_w3s", b"mod_w3s 3 c8 ff 61 00 00 00"),
            // mod_w1s: MOVE = iv+s2v+vl ; PRE mod_w1s 7 ef bf bd ef bf bd
            //   iv: mod_w1s 3 c8 ff 61 00 00 00
            //   s2v: mod_w1s 3 c8 ff 61 00 00 00
            //   vl: mod_w1s 3 c8 ff 61 00 00 00
            ("mod_w1s", b"mod_w1s 3 c8 ff 61 00 00 00"),
            // mod_m3s: MOVE = iv+s2v+vl ; PRE mod_m3s 7 ef bf bd ef bf bd
            //   iv: mod_m3s 3 c8 ff 61 00 00 00
            //   s2v: mod_m3s 3 c8 ff 61 00 00 00
            //   vl: mod_m3s 3 c8 ff 61 00 00 00
            ("mod_m3s", b"mod_m3s 3 c8 ff 61 00 00 00"),
            // mod_z3s: MOVE = iv+s2v+vl ; PRE mod_z3s 7 ef bf bd ef bf bd
            //   iv: mod_z3s 3 c8 ff 61 00 00 00
            //   s2v: mod_z3s 3 c8 ff 61 00 00 00
            //   vl: mod_z3s 3 c8 ff 61 00 00 00
            ("mod_z3s", b"mod_z3s 3 c8 ff 61 00 00 00"),
            // mod_w3S: MOVE = iv+s2v+vl ; PRE mod_w3S 7 ef bf bd ef bf bd
            //   iv: mod_w3S 3 c8 ff 61 00 00 00
            //   s2v: mod_w3S 3 c8 ff 61 00 00 00
            //   vl: mod_w3S 3 c8 ff 61 00 00 00
            ("mod_w3S", b"mod_w3S 3 c8 ff 61 00 00 00"),
            // mod_sc: MOVE = iv+s2v+vl ; PRE mod_sc 8 ef bf bd ef bf bd
            //   iv: mod_sc 4 c8 ff 61 41 00 00
            //   s2v: mod_sc 4 c8 ff 61 41 00 00
            //   vl: mod_sc 4 c8 ff 61 41 00 00
            ("mod_sc", b"mod_sc 4 c8 ff 61 41 00 00"),
            // mod_s3s: MOVE = iv+s2v+vl ; PRE mod_s3s 9 61 62 ef bf bd ef
            //   iv: mod_s3s 5 61 62 c8 ff 61 00
            //   s2v: mod_s3s 5 61 62 c8 ff 61 00
            //   vl: mod_s3s 5 61 62 c8 ff 61 00
            ("mod_s3s", b"mod_s3s 5 61 62 c8 ff 61 00"),
            // mod_u3s: MOVE = iv+s2v+vl ; PRE mod_u3s 4 20 c3 88 61 00 00
            //   iv: mod_u3s 3 c3 88 61 00 00 00
            //   s2v: mod_u3s 3 c3 88 61 00 00 00
            //   vl: mod_u3s 3 c3 88 61 00 00 00
            ("mod_u3s", b"mod_u3s 3 c3 88 61 00 00 00"),
            // mod_u4s: MOVE = iv+s2v+vl ; PRE mod_u4s 5 20 20 c3 88 61 00
            //   iv: mod_u4s 4 20 c3 88 61 00 00
            //   s2v: mod_u4s 4 20 c3 88 61 00 00
            //   vl: mod_u4s 4 20 c3 88 61 00 00
            ("mod_u4s", b"mod_u4s 4 20 c3 88 61 00 00"),
            // mod_um4: MOVE = iv+s2v+vl ; PRE mod_um4 5 c3 88 61 20 20 00
            //   iv: mod_um4 4 c3 88 61 20 00 00
            //   s2v: mod_um4 4 c3 88 61 20 00 00
            //   vl: mod_um4 4 c3 88 61 20 00 00
            ("mod_um4", b"mod_um4 4 c3 88 61 20 00 00"),
            // mod_e3s: MOVE = iv+s2v+vl ; PRE mod_e3s 5 20 20 e2 82 ac 00
            //   iv: mod_e3s 3 e2 82 ac 00 00 00
            //   s2v: mod_e3s 3 e2 82 ac 00 00 00
            //   vl: mod_e3s 3 e2 82 ac 00 00 00
            ("mod_e3s", b"mod_e3s 3 e2 82 ac 00 00 00"),
            // mod_e5s: MOVE = iv+s2v+vl ; PRE mod_e5s 7 20 20 20 20 e2 82
            //   iv: mod_e5s 5 20 20 e2 82 ac 00
            //   s2v: mod_e5s 5 20 20 e2 82 ac 00
            //   vl: mod_e5s 5 20 20 e2 82 ac 00
            ("mod_e5s", b"mod_e5s 5 20 20 e2 82 ac 00"),
            // mod_p0s: MOVE = iv+s2v+vl ; PRE mod_p0s 7 ef bf bd ef bf bd
            //   iv: mod_p0s 3 c8 ff 61 00 00 00
            //   s2v: mod_p0s 3 c8 ff 61 00 00 00
            //   vl: mod_p0s 3 c8 ff 61 00 00 00
            ("mod_p0s", b"mod_p0s 3 c8 ff 61 00 00 00"),
            // mod_pS: MOVE = iv+s2v+vl ; PRE mod_pS 7 ef bf bd ef bf bd
            //   iv: mod_pS 3 c8 ff 61 00 00 00
            //   s2v: mod_pS 3 c8 ff 61 00 00 00
            //   vl: mod_pS 3 c8 ff 61 00 00 00
            ("mod_pS", b"mod_pS 3 c8 ff 61 00 00 00"),
            // mod_pms: MOVE = iv+s2v+vl ; PRE mod_pms 7 ef bf bd ef bf bd
            //   iv: mod_pms 3 c8 ff 61 00 00 00
            //   s2v: mod_pms 3 c8 ff 61 00 00 00
            //   vl: mod_pms 3 c8 ff 61 00 00 00
            ("mod_pms", b"mod_pms 3 c8 ff 61 00 00 00"),
            // mod_ps: MOVE = iv+s2v+vl ; PRE mod_ps 7 ef bf bd ef bf bd
            //   iv: mod_ps 3 c8 ff 61 00 00 00
            //   s2v: mod_ps 3 c8 ff 61 00 00 00
            //   vl: mod_ps 3 c8 ff 61 00 00 00
            ("mod_ps", b"mod_ps 3 c8 ff 61 00 00 00"),
            // mod_cat: MOVE = iv+s2v+vl ; PRE mod_cat 8 ef bf bd ef bf bd
            //   iv: mod_cat 4 c8 ff 61 21 00 00
            //   s2v: mod_cat 4 c8 ff 61 21 00 00
            //   vl: mod_cat 4 c8 ff 61 21 00 00
            ("mod_cat", b"mod_cat 4 c8 ff 61 21 00 00"),
            // mod_nst2: MOVE = iv+s2v+vl ; PRE mod_nst2 7 ef bf bd ef bf bd
            //   iv: mod_nst2 3 c8 ff 61 00 00 00
            //   s2v: mod_nst2 3 c8 ff 61 00 00 00
            //   vl: mod_nst2 3 c8 ff 61 00 00 00
            ("mod_nst2", b"mod_nst2 3 c8 ff 61 00 00 00"),
            // fnb_w3s: MOVE = iv+s2v+vl ; PRE fnb_w3s 7 ef bf bd ef bf bd
            //   iv: fnb_w3s 3 c8 ff 61 00 00 00
            //   s2v: fnb_w3s 3 c8 ff 61 00 00 00
            //   vl: fnb_w3s 3 c8 ff 61 00 00 00
            ("fnb_w3s", b"fnb_w3s 3 c8 ff 61 00 00 00"),
            // fnb_w1s: MOVE = iv+s2v+vl ; PRE fnb_w1s 7 ef bf bd ef bf bd
            //   iv: fnb_w1s 3 c8 ff 61 00 00 00
            //   s2v: fnb_w1s 3 c8 ff 61 00 00 00
            //   vl: fnb_w1s 3 c8 ff 61 00 00 00
            ("fnb_w1s", b"fnb_w1s 3 c8 ff 61 00 00 00"),
            // fnb_m3s: MOVE = iv+s2v+vl ; PRE fnb_m3s 7 ef bf bd ef bf bd
            //   iv: fnb_m3s 3 c8 ff 61 00 00 00
            //   s2v: fnb_m3s 3 c8 ff 61 00 00 00
            //   vl: fnb_m3s 3 c8 ff 61 00 00 00
            ("fnb_m3s", b"fnb_m3s 3 c8 ff 61 00 00 00"),
            // fnb_z3s: MOVE = iv+s2v+vl ; PRE fnb_z3s 7 ef bf bd ef bf bd
            //   iv: fnb_z3s 3 c8 ff 61 00 00 00
            //   s2v: fnb_z3s 3 c8 ff 61 00 00 00
            //   vl: fnb_z3s 3 c8 ff 61 00 00 00
            ("fnb_z3s", b"fnb_z3s 3 c8 ff 61 00 00 00"),
            // fnb_w3S: MOVE = iv+s2v+vl ; PRE fnb_w3S 7 ef bf bd ef bf bd
            //   iv: fnb_w3S 3 c8 ff 61 00 00 00
            //   s2v: fnb_w3S 3 c8 ff 61 00 00 00
            //   vl: fnb_w3S 3 c8 ff 61 00 00 00
            ("fnb_w3S", b"fnb_w3S 3 c8 ff 61 00 00 00"),
            // fnb_sc: MOVE = iv+s2v+vl ; PRE fnb_sc 8 ef bf bd ef bf bd
            //   iv: fnb_sc 4 c8 ff 61 41 00 00
            //   s2v: fnb_sc 4 c8 ff 61 41 00 00
            //   vl: fnb_sc 4 c8 ff 61 41 00 00
            ("fnb_sc", b"fnb_sc 4 c8 ff 61 41 00 00"),
            // fnb_s3s: MOVE = iv+s2v+vl ; PRE fnb_s3s 9 61 62 ef bf bd ef
            //   iv: fnb_s3s 5 61 62 c8 ff 61 00
            //   s2v: fnb_s3s 5 61 62 c8 ff 61 00
            //   vl: fnb_s3s 5 61 62 c8 ff 61 00
            ("fnb_s3s", b"fnb_s3s 5 61 62 c8 ff 61 00"),
            // fnb_u3s: MOVE = iv+s2v+vl ; PRE fnb_u3s 4 20 c3 88 61 00 00
            //   iv: fnb_u3s 3 c3 88 61 00 00 00
            //   s2v: fnb_u3s 3 c3 88 61 00 00 00
            //   vl: fnb_u3s 3 c3 88 61 00 00 00
            ("fnb_u3s", b"fnb_u3s 3 c3 88 61 00 00 00"),
            // fnb_u4s: MOVE = iv+s2v+vl ; PRE fnb_u4s 5 20 20 c3 88 61 00
            //   iv: fnb_u4s 4 20 c3 88 61 00 00
            //   s2v: fnb_u4s 4 20 c3 88 61 00 00
            //   vl: fnb_u4s 4 20 c3 88 61 00 00
            ("fnb_u4s", b"fnb_u4s 4 20 c3 88 61 00 00"),
            // fnb_um4: MOVE = iv+s2v+vl ; PRE fnb_um4 5 c3 88 61 20 20 00
            //   iv: fnb_um4 4 c3 88 61 20 00 00
            //   s2v: fnb_um4 4 c3 88 61 20 00 00
            //   vl: fnb_um4 4 c3 88 61 20 00 00
            ("fnb_um4", b"fnb_um4 4 c3 88 61 20 00 00"),
            // fnb_e3s: MOVE = iv+s2v+vl ; PRE fnb_e3s 5 20 20 e2 82 ac 00
            //   iv: fnb_e3s 3 e2 82 ac 00 00 00
            //   s2v: fnb_e3s 3 e2 82 ac 00 00 00
            //   vl: fnb_e3s 3 e2 82 ac 00 00 00
            ("fnb_e3s", b"fnb_e3s 3 e2 82 ac 00 00 00"),
            // fnb_e5s: MOVE = iv+s2v+vl ; PRE fnb_e5s 7 20 20 20 20 e2 82
            //   iv: fnb_e5s 5 20 20 e2 82 ac 00
            //   s2v: fnb_e5s 5 20 20 e2 82 ac 00
            //   vl: fnb_e5s 5 20 20 e2 82 ac 00
            ("fnb_e5s", b"fnb_e5s 5 20 20 e2 82 ac 00"),
            // fnb_p0s: MOVE = iv+s2v+vl ; PRE fnb_p0s 7 ef bf bd ef bf bd
            //   iv: fnb_p0s 3 c8 ff 61 00 00 00
            //   s2v: fnb_p0s 3 c8 ff 61 00 00 00
            //   vl: fnb_p0s 3 c8 ff 61 00 00 00
            ("fnb_p0s", b"fnb_p0s 3 c8 ff 61 00 00 00"),
            // fnb_pS: MOVE = iv+s2v+vl ; PRE fnb_pS 7 ef bf bd ef bf bd
            //   iv: fnb_pS 3 c8 ff 61 00 00 00
            //   s2v: fnb_pS 3 c8 ff 61 00 00 00
            //   vl: fnb_pS 3 c8 ff 61 00 00 00
            ("fnb_pS", b"fnb_pS 3 c8 ff 61 00 00 00"),
            // fnb_pms: MOVE = iv+s2v+vl ; PRE fnb_pms 7 ef bf bd ef bf bd
            //   iv: fnb_pms 3 c8 ff 61 00 00 00
            //   s2v: fnb_pms 3 c8 ff 61 00 00 00
            //   vl: fnb_pms 3 c8 ff 61 00 00 00
            ("fnb_pms", b"fnb_pms 3 c8 ff 61 00 00 00"),
            // fnb_ps: MOVE = iv+s2v+vl ; PRE fnb_ps 7 ef bf bd ef bf bd
            //   iv: fnb_ps 3 c8 ff 61 00 00 00
            //   s2v: fnb_ps 3 c8 ff 61 00 00 00
            //   vl: fnb_ps 3 c8 ff 61 00 00 00
            ("fnb_ps", b"fnb_ps 3 c8 ff 61 00 00 00"),
            // fnb_cat: MOVE = iv+s2v+vl ; PRE fnb_cat 8 ef bf bd ef bf bd
            //   iv: fnb_cat 4 c8 ff 61 21 00 00
            //   s2v: fnb_cat 4 c8 ff 61 21 00 00
            //   vl: fnb_cat 4 c8 ff 61 21 00 00
            ("fnb_cat", b"fnb_cat 4 c8 ff 61 21 00 00"),
            // ret_w3s: MOVE = iv+s2v+vl ; PRE ret_w3s 7 ef bf bd ef bf bd
            //   iv: ret_w3s 3 c8 ff 61 00 00 00
            //   s2v: ret_w3s 3 c8 ff 61 00 00 00
            //   vl: ret_w3s 3 c8 ff 61 00 00 00
            ("ret_w3s", b"ret_w3s 3 c8 ff 61 00 00 00"),
            // ret_w1s: MOVE = iv+s2v+vl ; PRE ret_w1s 7 ef bf bd ef bf bd
            //   iv: ret_w1s 3 c8 ff 61 00 00 00
            //   s2v: ret_w1s 3 c8 ff 61 00 00 00
            //   vl: ret_w1s 3 c8 ff 61 00 00 00
            ("ret_w1s", b"ret_w1s 3 c8 ff 61 00 00 00"),
            // ret_m3s: MOVE = iv+s2v+vl ; PRE ret_m3s 7 ef bf bd ef bf bd
            //   iv: ret_m3s 3 c8 ff 61 00 00 00
            //   s2v: ret_m3s 3 c8 ff 61 00 00 00
            //   vl: ret_m3s 3 c8 ff 61 00 00 00
            ("ret_m3s", b"ret_m3s 3 c8 ff 61 00 00 00"),
            // ret_z3s: MOVE = iv+s2v+vl ; PRE ret_z3s 7 ef bf bd ef bf bd
            //   iv: ret_z3s 3 c8 ff 61 00 00 00
            //   s2v: ret_z3s 3 c8 ff 61 00 00 00
            //   vl: ret_z3s 3 c8 ff 61 00 00 00
            ("ret_z3s", b"ret_z3s 3 c8 ff 61 00 00 00"),
            // ret_w3S: MOVE = iv+s2v+vl ; PRE ret_w3S 7 ef bf bd ef bf bd
            //   iv: ret_w3S 3 c8 ff 61 00 00 00
            //   s2v: ret_w3S 3 c8 ff 61 00 00 00
            //   vl: ret_w3S 3 c8 ff 61 00 00 00
            ("ret_w3S", b"ret_w3S 3 c8 ff 61 00 00 00"),
            // ret_sc: MOVE = iv+s2v+vl ; PRE ret_sc 8 ef bf bd ef bf bd
            //   iv: ret_sc 4 c8 ff 61 41 00 00
            //   s2v: ret_sc 4 c8 ff 61 41 00 00
            //   vl: ret_sc 4 c8 ff 61 41 00 00
            ("ret_sc", b"ret_sc 4 c8 ff 61 41 00 00"),
            // ret_s3s: MOVE = iv+s2v+vl ; PRE ret_s3s 9 61 62 ef bf bd ef
            //   iv: ret_s3s 5 61 62 c8 ff 61 00
            //   s2v: ret_s3s 5 61 62 c8 ff 61 00
            //   vl: ret_s3s 5 61 62 c8 ff 61 00
            ("ret_s3s", b"ret_s3s 5 61 62 c8 ff 61 00"),
            // ret_u3s: MOVE = iv+s2v+vl ; PRE ret_u3s 4 20 c3 88 61 00 00
            //   iv: ret_u3s 3 c3 88 61 00 00 00
            //   s2v: ret_u3s 3 c3 88 61 00 00 00
            //   vl: ret_u3s 3 c3 88 61 00 00 00
            ("ret_u3s", b"ret_u3s 3 c3 88 61 00 00 00"),
            // ret_u4s: MOVE = iv+s2v+vl ; PRE ret_u4s 5 20 20 c3 88 61 00
            //   iv: ret_u4s 4 20 c3 88 61 00 00
            //   s2v: ret_u4s 4 20 c3 88 61 00 00
            //   vl: ret_u4s 4 20 c3 88 61 00 00
            ("ret_u4s", b"ret_u4s 4 20 c3 88 61 00 00"),
            // ret_um4: MOVE = iv+s2v+vl ; PRE ret_um4 5 c3 88 61 20 20 00
            //   iv: ret_um4 4 c3 88 61 20 00 00
            //   s2v: ret_um4 4 c3 88 61 20 00 00
            //   vl: ret_um4 4 c3 88 61 20 00 00
            ("ret_um4", b"ret_um4 4 c3 88 61 20 00 00"),
            // ret_e3s: MOVE = iv+s2v+vl ; PRE ret_e3s 5 20 20 e2 82 ac 00
            //   iv: ret_e3s 3 e2 82 ac 00 00 00
            //   s2v: ret_e3s 3 e2 82 ac 00 00 00
            //   vl: ret_e3s 3 e2 82 ac 00 00 00
            ("ret_e3s", b"ret_e3s 3 e2 82 ac 00 00 00"),
            // ret_e5s: MOVE = iv+s2v+vl ; PRE ret_e5s 7 20 20 20 20 e2 82
            //   iv: ret_e5s 5 20 20 e2 82 ac 00
            //   s2v: ret_e5s 5 20 20 e2 82 ac 00
            //   vl: ret_e5s 5 20 20 e2 82 ac 00
            ("ret_e5s", b"ret_e5s 5 20 20 e2 82 ac 00"),
            // ret_p0s: MOVE = iv+s2v+vl ; PRE ret_p0s 7 ef bf bd ef bf bd
            //   iv: ret_p0s 3 c8 ff 61 00 00 00
            //   s2v: ret_p0s 3 c8 ff 61 00 00 00
            //   vl: ret_p0s 3 c8 ff 61 00 00 00
            ("ret_p0s", b"ret_p0s 3 c8 ff 61 00 00 00"),
            // ret_pS: MOVE = iv+s2v+vl ; PRE ret_pS 7 ef bf bd ef bf bd
            //   iv: ret_pS 3 c8 ff 61 00 00 00
            //   s2v: ret_pS 3 c8 ff 61 00 00 00
            //   vl: ret_pS 3 c8 ff 61 00 00 00
            ("ret_pS", b"ret_pS 3 c8 ff 61 00 00 00"),
            // ret_pms: MOVE = iv+s2v+vl ; PRE ret_pms 7 ef bf bd ef bf bd
            //   iv: ret_pms 3 c8 ff 61 00 00 00
            //   s2v: ret_pms 3 c8 ff 61 00 00 00
            //   vl: ret_pms 3 c8 ff 61 00 00 00
            ("ret_pms", b"ret_pms 3 c8 ff 61 00 00 00"),
            // ret_ps: MOVE = iv+s2v+vl ; PRE ret_ps 7 ef bf bd ef bf bd
            //   iv: ret_ps 3 c8 ff 61 00 00 00
            //   s2v: ret_ps 3 c8 ff 61 00 00 00
            //   vl: ret_ps 3 c8 ff 61 00 00 00
            ("ret_ps", b"ret_ps 3 c8 ff 61 00 00 00"),
            // sfm_w3s: MOVE = iv+s2v+vl ; PRE sfm_w3s 7 ef bf bd ef bf bd
            //   iv: sfm_w3s 3 c8 ff 61 00 00 00
            //   s2v: sfm_w3s 3 c8 ff 61 00 00 00
            //   vl: sfm_w3s 3 c8 ff 61 00 00 00
            ("sfm_w3s", b"sfm_w3s 3 c8 ff 61 00 00 00"),
            // sfm_w1s: MOVE = iv+s2v+vl ; PRE sfm_w1s 7 ef bf bd ef bf bd
            //   iv: sfm_w1s 3 c8 ff 61 00 00 00
            //   s2v: sfm_w1s 3 c8 ff 61 00 00 00
            //   vl: sfm_w1s 3 c8 ff 61 00 00 00
            ("sfm_w1s", b"sfm_w1s 3 c8 ff 61 00 00 00"),
            // sfm_m3s: MOVE = iv+s2v+vl ; PRE sfm_m3s 7 ef bf bd ef bf bd
            //   iv: sfm_m3s 3 c8 ff 61 00 00 00
            //   s2v: sfm_m3s 3 c8 ff 61 00 00 00
            //   vl: sfm_m3s 3 c8 ff 61 00 00 00
            ("sfm_m3s", b"sfm_m3s 3 c8 ff 61 00 00 00"),
            // sfm_z3s: MOVE = iv+s2v+vl ; PRE sfm_z3s 7 ef bf bd ef bf bd
            //   iv: sfm_z3s 3 c8 ff 61 00 00 00
            //   s2v: sfm_z3s 3 c8 ff 61 00 00 00
            //   vl: sfm_z3s 3 c8 ff 61 00 00 00
            ("sfm_z3s", b"sfm_z3s 3 c8 ff 61 00 00 00"),
            // sfm_w3S: MOVE = iv+s2v+vl ; PRE sfm_w3S 7 ef bf bd ef bf bd
            //   iv: sfm_w3S 3 c8 ff 61 00 00 00
            //   s2v: sfm_w3S 3 c8 ff 61 00 00 00
            //   vl: sfm_w3S 3 c8 ff 61 00 00 00
            ("sfm_w3S", b"sfm_w3S 3 c8 ff 61 00 00 00"),
            // sfm_sc: MOVE = iv+s2v+vl ; PRE sfm_sc 8 ef bf bd ef bf bd
            //   iv: sfm_sc 4 c8 ff 61 41 00 00
            //   s2v: sfm_sc 4 c8 ff 61 41 00 00
            //   vl: sfm_sc 4 c8 ff 61 41 00 00
            ("sfm_sc", b"sfm_sc 4 c8 ff 61 41 00 00"),
            // sfm_s3s: MOVE = iv+s2v+vl ; PRE sfm_s3s 9 61 62 ef bf bd ef
            //   iv: sfm_s3s 5 61 62 c8 ff 61 00
            //   s2v: sfm_s3s 5 61 62 c8 ff 61 00
            //   vl: sfm_s3s 5 61 62 c8 ff 61 00
            ("sfm_s3s", b"sfm_s3s 5 61 62 c8 ff 61 00"),
            // sfm_u3s: MOVE = iv+s2v+vl ; PRE sfm_u3s 4 20 c3 88 61 00 00
            //   iv: sfm_u3s 3 c3 88 61 00 00 00
            //   s2v: sfm_u3s 3 c3 88 61 00 00 00
            //   vl: sfm_u3s 3 c3 88 61 00 00 00
            ("sfm_u3s", b"sfm_u3s 3 c3 88 61 00 00 00"),
            // sfm_u4s: MOVE = iv+s2v+vl ; PRE sfm_u4s 5 20 20 c3 88 61 00
            //   iv: sfm_u4s 4 20 c3 88 61 00 00
            //   s2v: sfm_u4s 4 20 c3 88 61 00 00
            //   vl: sfm_u4s 4 20 c3 88 61 00 00
            ("sfm_u4s", b"sfm_u4s 4 20 c3 88 61 00 00"),
            // sfm_um4: MOVE = iv+s2v+vl ; PRE sfm_um4 5 c3 88 61 20 20 00
            //   iv: sfm_um4 4 c3 88 61 20 00 00
            //   s2v: sfm_um4 4 c3 88 61 20 00 00
            //   vl: sfm_um4 4 c3 88 61 20 00 00
            ("sfm_um4", b"sfm_um4 4 c3 88 61 20 00 00"),
            // sfm_e3s: MOVE = iv+s2v+vl ; PRE sfm_e3s 5 20 20 e2 82 ac 00
            //   iv: sfm_e3s 3 e2 82 ac 00 00 00
            //   s2v: sfm_e3s 3 e2 82 ac 00 00 00
            //   vl: sfm_e3s 3 e2 82 ac 00 00 00
            ("sfm_e3s", b"sfm_e3s 3 e2 82 ac 00 00 00"),
            // sfm_e5s: MOVE = iv+s2v+vl ; PRE sfm_e5s 7 20 20 20 20 e2 82
            //   iv: sfm_e5s 5 20 20 e2 82 ac 00
            //   s2v: sfm_e5s 5 20 20 e2 82 ac 00
            //   vl: sfm_e5s 5 20 20 e2 82 ac 00
            ("sfm_e5s", b"sfm_e5s 5 20 20 e2 82 ac 00"),
            // sfm_p0s: MOVE = iv+s2v+vl ; PRE sfm_p0s 7 ef bf bd ef bf bd
            //   iv: sfm_p0s 3 c8 ff 61 00 00 00
            //   s2v: sfm_p0s 3 c8 ff 61 00 00 00
            //   vl: sfm_p0s 3 c8 ff 61 00 00 00
            ("sfm_p0s", b"sfm_p0s 3 c8 ff 61 00 00 00"),
            // sfm_pS: MOVE = iv+s2v+vl ; PRE sfm_pS 7 ef bf bd ef bf bd
            //   iv: sfm_pS 3 c8 ff 61 00 00 00
            //   s2v: sfm_pS 3 c8 ff 61 00 00 00
            //   vl: sfm_pS 3 c8 ff 61 00 00 00
            ("sfm_pS", b"sfm_pS 3 c8 ff 61 00 00 00"),
            // sfm_pms: MOVE = iv+s2v+vl ; PRE sfm_pms 7 ef bf bd ef bf bd
            //   iv: sfm_pms 3 c8 ff 61 00 00 00
            //   s2v: sfm_pms 3 c8 ff 61 00 00 00
            //   vl: sfm_pms 3 c8 ff 61 00 00 00
            ("sfm_pms", b"sfm_pms 3 c8 ff 61 00 00 00"),
            // sfm_ps: MOVE = iv+s2v+vl ; PRE sfm_ps 7 ef bf bd ef bf bd
            //   iv: sfm_ps 3 c8 ff 61 00 00 00
            //   s2v: sfm_ps 3 c8 ff 61 00 00 00
            //   vl: sfm_ps 3 c8 ff 61 00 00 00
            ("sfm_ps", b"sfm_ps 3 c8 ff 61 00 00 00"),
        ],
        &[],
    );
}

#[test]
fn d_d1_val_stays() {
    check(
        &Cell {
            src: D_D1_VAL_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // mod_a5s: STAY = iv+s2v+vl ; PRE mod_a5s 5 20 20 20 61 62 00
            //   iv: mod_a5s 5 20 20 20 61 62 00
            //   s2v: mod_a5s 5 20 20 20 61 62 00
            //   vl: mod_a5s 5 20 20 20 61 62 00
            ("mod_a5s", b"mod_a5s 5 20 20 20 61 62 00"),
            // mod_a2s: STAY = iv+s2v+vl ; PRE mod_a2s 2 61 62 00 00 00 00
            //   iv: mod_a2s 2 61 62 00 00 00 00
            //   s2v: mod_a2s 2 61 62 00 00 00 00
            //   vl: mod_a2s 2 61 62 00 00 00 00
            ("mod_a2s", b"mod_a2s 2 61 62 00 00 00 00"),
            // fnb_a5s: STAY = iv+s2v+vl ; PRE fnb_a5s 5 20 20 20 61 62 00
            //   iv: fnb_a5s 5 20 20 20 61 62 00
            //   s2v: fnb_a5s 5 20 20 20 61 62 00
            //   vl: fnb_a5s 5 20 20 20 61 62 00
            ("fnb_a5s", b"fnb_a5s 5 20 20 20 61 62 00"),
            // fnb_a2s: STAY = iv+s2v+vl ; PRE fnb_a2s 2 61 62 00 00 00 00
            //   iv: fnb_a2s 2 61 62 00 00 00 00
            //   s2v: fnb_a2s 2 61 62 00 00 00 00
            //   vl: fnb_a2s 2 61 62 00 00 00 00
            ("fnb_a2s", b"fnb_a2s 2 61 62 00 00 00 00"),
            // ret_a5s: STAY = iv+s2v+vl ; PRE ret_a5s 5 20 20 20 61 62 00
            //   iv: ret_a5s 5 20 20 20 61 62 00
            //   s2v: ret_a5s 5 20 20 20 61 62 00
            //   vl: ret_a5s 5 20 20 20 61 62 00
            ("ret_a5s", b"ret_a5s 5 20 20 20 61 62 00"),
            // ret_a2s: STAY = iv+s2v+vl ; PRE ret_a2s 2 61 62 00 00 00 00
            //   iv: ret_a2s 2 61 62 00 00 00 00
            //   s2v: ret_a2s 2 61 62 00 00 00 00
            //   vl: ret_a2s 2 61 62 00 00 00 00
            ("ret_a2s", b"ret_a2s 2 61 62 00 00 00 00"),
            // sfm_a5s: STAY = iv+s2v+vl ; PRE sfm_a5s 5 20 20 20 61 62 00
            //   iv: sfm_a5s 5 20 20 20 61 62 00
            //   s2v: sfm_a5s 5 20 20 20 61 62 00
            //   vl: sfm_a5s 5 20 20 20 61 62 00
            ("sfm_a5s", b"sfm_a5s 5 20 20 20 61 62 00"),
            // sfm_a2s: STAY = iv+s2v+vl ; PRE sfm_a2s 2 61 62 00 00 00 00
            //   iv: sfm_a2s 2 61 62 00 00 00 00
            //   s2v: sfm_a2s 2 61 62 00 00 00 00
            //   vl: sfm_a2s 2 61 62 00 00 00 00
            ("sfm_a2s", b"sfm_a2s 2 61 62 00 00 00 00"),
        ],
        &[],
    );
}

const D_D1_STR_SRC: &str = r#"module top;
  string t_ab = "ab", t_hi = "\310\377a", s_u8 = "\303\210a", s_e3 = "\342\202\254", u;
  logic [7:0] b_a = 8'h41;
  integer fd;
  initial begin
    fd = $fopen("fo.bin", "wb");
    $display("dsp_w3s [%3s]", t_hi);
    $display("dsp_w1s [%1s]", t_hi);
    $display("dsp_m3s [%-3s]", t_hi);
    $display("dsp_z3s [%03s]", t_hi);
    $display("dsp_w3S [%3S]", t_hi);
    $display("dsp_sc [%s%c]", t_hi, b_a);
    $display("dsp_s3s [%s%3s]", t_ab, t_hi);
    $display("dsp_u3s [%3s]", s_u8);
    $display("dsp_u4s [%4s]", s_u8);
    $display("dsp_um4 [%-4s]", s_u8);
    $display("dsp_e3s [%3s]", s_e3);
    $display("dsp_e5s [%5s]", s_e3);
    $display("dsp_p0s [%0s]", t_hi);
    $display("dsp_pS [%S]", t_hi);
    $display("dsp_pms [%-s]", t_hi);
    $display("dsp_ps [%s]", t_hi);
    $display("dsp_a5s [%5s]", t_ab);
    $display("dsp_a2s [%2s]", t_ab);
    $display("dsp_nst [%s]", $sformatf("%3s", t_hi));
    $fwrite(fd, "fw_w3s [%3s]\n", t_hi);
    $fwrite(fd, "fw_w1s [%1s]\n", t_hi);
    $fwrite(fd, "fw_m3s [%-3s]\n", t_hi);
    $fwrite(fd, "fw_z3s [%03s]\n", t_hi);
    $fwrite(fd, "fw_w3S [%3S]\n", t_hi);
    $fwrite(fd, "fw_sc [%s%c]\n", t_hi, b_a);
    $fwrite(fd, "fw_s3s [%s%3s]\n", t_ab, t_hi);
    $fwrite(fd, "fw_u3s [%3s]\n", s_u8);
    $fwrite(fd, "fw_u4s [%4s]\n", s_u8);
    $fwrite(fd, "fw_um4 [%-4s]\n", s_u8);
    $fwrite(fd, "fw_e3s [%3s]\n", s_e3);
    $fwrite(fd, "fw_e5s [%5s]\n", s_e3);
    $fwrite(fd, "fw_p0s [%0s]\n", t_hi);
    $fwrite(fd, "fw_pS [%S]\n", t_hi);
    $fwrite(fd, "fw_pms [%-s]\n", t_hi);
    $fwrite(fd, "fw_ps [%s]\n", t_hi);
    $fwrite(fd, "fw_a5s [%5s]\n", t_ab);
    $fwrite(fd, "fw_a2s [%2s]\n", t_ab);
    $fclose(fd);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn d_d1_str_moves() {
    check(
        &Cell { src: D_D1_STR_SRC, inputs: &[], plus: None, mode: Mode::Word, staged: true },
        &[
            // dsp_w3s: MOVE = iv+s2v+vl ; PRE dsp_w3s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_w3s [\xc8\xffa]
            //   s2v: dsp_w3s [\xc8\xffa]
            //   vl: dsp_w3s [\xc8\xffa]
            ("dsp_w3s", b"dsp_w3s [\xc8\xffa]"),
            // dsp_w1s: MOVE = iv+s2v+vl ; PRE dsp_w1s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_w1s [\xc8\xffa]
            //   s2v: dsp_w1s [\xc8\xffa]
            //   vl: dsp_w1s [\xc8\xffa]
            ("dsp_w1s", b"dsp_w1s [\xc8\xffa]"),
            // dsp_m3s: MOVE = iv+s2v+vl ; PRE dsp_m3s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_m3s [\xc8\xffa]
            //   s2v: dsp_m3s [\xc8\xffa]
            //   vl: dsp_m3s [\xc8\xffa]
            ("dsp_m3s", b"dsp_m3s [\xc8\xffa]"),
            // dsp_z3s: MOVE = iv+s2v+vl ; PRE dsp_z3s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_z3s [\xc8\xffa]
            //   s2v: dsp_z3s [\xc8\xffa]
            //   vl: dsp_z3s [\xc8\xffa]
            ("dsp_z3s", b"dsp_z3s [\xc8\xffa]"),
            // dsp_w3S: MOVE = iv+s2v+vl ; PRE dsp_w3S [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_w3S [\xc8\xffa]
            //   s2v: dsp_w3S [\xc8\xffa]
            //   vl: dsp_w3S [\xc8\xffa]
            ("dsp_w3S", b"dsp_w3S [\xc8\xffa]"),
            // dsp_sc: MOVE = iv+s2v+vl ; PRE dsp_sc [\xef\xbf\xbd\xef\xbf\xbdaA]
            //   iv: dsp_sc [\xc8\xffaA]
            //   s2v: dsp_sc [\xc8\xffaA]
            //   vl: dsp_sc [\xc8\xffaA]
            ("dsp_sc", b"dsp_sc [\xc8\xffaA]"),
            // dsp_s3s: MOVE = iv+s2v+vl ; PRE dsp_s3s [ab\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_s3s [ab\xc8\xffa]
            //   s2v: dsp_s3s [ab\xc8\xffa]
            //   vl: dsp_s3s [ab\xc8\xffa]
            ("dsp_s3s", b"dsp_s3s [ab\xc8\xffa]"),
            // dsp_u3s: MOVE = iv+s2v+vl ; PRE dsp_u3s [ \xc3\x88a]
            //   iv: dsp_u3s [\xc3\x88a]
            //   s2v: dsp_u3s [\xc3\x88a]
            //   vl: dsp_u3s [\xc3\x88a]
            ("dsp_u3s", b"dsp_u3s [\xc3\x88a]"),
            // dsp_u4s: MOVE = iv+s2v+vl ; PRE dsp_u4s [  \xc3\x88a]
            //   iv: dsp_u4s [ \xc3\x88a]
            //   s2v: dsp_u4s [ \xc3\x88a]
            //   vl: dsp_u4s [ \xc3\x88a]
            ("dsp_u4s", b"dsp_u4s [ \xc3\x88a]"),
            // dsp_um4: MOVE = iv+s2v+vl ; PRE dsp_um4 [\xc3\x88a  ]
            //   iv: dsp_um4 [\xc3\x88a ]
            //   s2v: dsp_um4 [\xc3\x88a ]
            //   vl: dsp_um4 [\xc3\x88a ]
            ("dsp_um4", b"dsp_um4 [\xc3\x88a ]"),
            // dsp_e3s: MOVE = iv+s2v+vl ; PRE dsp_e3s [  \xe2\x82\xac]
            //   iv: dsp_e3s [\xe2\x82\xac]
            //   s2v: dsp_e3s [\xe2\x82\xac]
            //   vl: dsp_e3s [\xe2\x82\xac]
            ("dsp_e3s", b"dsp_e3s [\xe2\x82\xac]"),
            // dsp_e5s: MOVE = iv+s2v+vl ; PRE dsp_e5s [    \xe2\x82\xac]
            //   iv: dsp_e5s [  \xe2\x82\xac]
            //   s2v: dsp_e5s [  \xe2\x82\xac]
            //   vl: dsp_e5s [  \xe2\x82\xac]
            ("dsp_e5s", b"dsp_e5s [  \xe2\x82\xac]"),
            // dsp_p0s: MOVE = iv+s2v+vl ; PRE dsp_p0s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_p0s [\xc8\xffa]
            //   s2v: dsp_p0s [\xc8\xffa]
            //   vl: dsp_p0s [\xc8\xffa]
            ("dsp_p0s", b"dsp_p0s [\xc8\xffa]"),
            // dsp_pS: MOVE = iv+s2v+vl ; PRE dsp_pS [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_pS [\xc8\xffa]
            //   s2v: dsp_pS [\xc8\xffa]
            //   vl: dsp_pS [\xc8\xffa]
            ("dsp_pS", b"dsp_pS [\xc8\xffa]"),
            // dsp_pms: MOVE = iv+s2v+vl ; PRE dsp_pms [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_pms [\xc8\xffa]
            //   s2v: dsp_pms [\xc8\xffa]
            //   vl: dsp_pms [\xc8\xffa]
            ("dsp_pms", b"dsp_pms [\xc8\xffa]"),
            // dsp_ps: MOVE = iv+s2v+vl ; PRE dsp_ps [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_ps [\xc8\xffa]
            //   s2v: dsp_ps [\xc8\xffa]
            //   vl: dsp_ps [\xc8\xffa]
            ("dsp_ps", b"dsp_ps [\xc8\xffa]"),
            // dsp_nst: MOVE = iv+s2v+vl ; PRE dsp_nst [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: dsp_nst [\xc8\xffa]
            //   s2v: dsp_nst [\xc8\xffa]
            //   vl: dsp_nst [\xc8\xffa]
            ("dsp_nst", b"dsp_nst [\xc8\xffa]"),
        ],
        &[
            // FILE:fo.bin: MOVE = iv+s2v+vl ; PRE fw_w3s [\xef\xbf\xbd\xef\xbf\xbda]\x0afw_w1s [\xef\xbf\xbd\xef\xbf\xbda]\x0afw_m3s [\xef\xbf\xbd\xef\xbf\xbda]\x0afw_z3s [\xef\xbf\xbd\xef\x
            ("fo.bin", b"fw_w3s [\xc8\xffa]\x0afw_w1s [\xc8\xffa]\x0afw_m3s [\xc8\xffa]\x0afw_z3s [\xc8\xffa]\x0afw_w3S [\xc8\xffa]\x0afw_sc [\xc8\xffaA]\x0afw_s3s [ab\xc8\xffa]\x0afw_u3s [\xc3\x88a]\x0afw_u4s [ \xc3\x88a]\x0afw_um4 [\xc3\x88a ]\x0afw_e3s [\xe2\x82\xac]\x0afw_e5s [  \xe2\x82\xac]\x0afw_p0s [\xc8\xffa]\x0afw_pS [\xc8\xffa]\x0afw_pms [\xc8\xffa]\x0afw_ps [\xc8\xffa]\x0afw_a5s [   ab]\x0afw_a2s [ab]\x0a"),
        ],
    );
}

#[test]
fn d_d1_str_stays() {
    check(
        &Cell {
            src: D_D1_STR_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // dsp_a5s: STAY = iv+s2v+vl ; PRE dsp_a5s [   ab]
            //   iv: dsp_a5s [   ab]
            //   s2v: dsp_a5s [   ab]
            //   vl: dsp_a5s [   ab]
            ("dsp_a5s", b"dsp_a5s [   ab]"),
            // dsp_a2s: STAY = iv+s2v+vl ; PRE dsp_a2s [ab]
            //   iv: dsp_a2s [ab]
            //   s2v: dsp_a2s [ab]
            //   vl: dsp_a2s [ab]
            ("dsp_a2s", b"dsp_a2s [ab]"),
        ],
        &[],
    );
}

const D_D1_SEV_SRC: &str = r#"module top;
  string t_ab = "ab", t_hi = "\310\377a", s_u8 = "\303\210a", s_e3 = "\342\202\254", u;
  logic [7:0] b_a = 8'h41;
  initial begin
    $info("sev_w3s [%3s]", t_hi);
    $info("sev_w1s [%1s]", t_hi);
    $info("sev_m3s [%-3s]", t_hi);
    $info("sev_z3s [%03s]", t_hi);
    $info("sev_w3S [%3S]", t_hi);
    $info("sev_sc [%s%c]", t_hi, b_a);
    $info("sev_s3s [%s%3s]", t_ab, t_hi);
    $info("sev_u3s [%3s]", s_u8);
    $info("sev_u4s [%4s]", s_u8);
    $info("sev_um4 [%-4s]", s_u8);
    $info("sev_e3s [%3s]", s_e3);
    $info("sev_e5s [%5s]", s_e3);
    $info("sev_p0s [%0s]", t_hi);
    $info("sev_pS [%S]", t_hi);
    $info("sev_pms [%-s]", t_hi);
    $info("sev_ps [%s]", t_hi);
    $info("sev_a5s [%5s]", t_ab);
    $info("sev_a2s [%2s]", t_ab);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn d_d1_sev_moves() {
    check(
        &Cell {
            src: D_D1_SEV_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // sev_w3s: MOVE = iv+s2v+vl ; PRE sev_w3s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_w3s [\xc8\xffa]
            //   s2v: sev_w3s [\xc8\xffa]
            //   vl: sev_w3s [\xc8\xffa]
            ("sev_w3s", b"sev_w3s [\xc8\xffa]"),
            // sev_w1s: MOVE = iv+s2v+vl ; PRE sev_w1s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_w1s [\xc8\xffa]
            //   s2v: sev_w1s [\xc8\xffa]
            //   vl: sev_w1s [\xc8\xffa]
            ("sev_w1s", b"sev_w1s [\xc8\xffa]"),
            // sev_m3s: MOVE = iv+s2v+vl ; PRE sev_m3s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_m3s [\xc8\xffa]
            //   s2v: sev_m3s [\xc8\xffa]
            //   vl: sev_m3s [\xc8\xffa]
            ("sev_m3s", b"sev_m3s [\xc8\xffa]"),
            // sev_z3s: MOVE = iv+s2v+vl ; PRE sev_z3s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_z3s [\xc8\xffa]
            //   s2v: sev_z3s [\xc8\xffa]
            //   vl: sev_z3s [\xc8\xffa]
            ("sev_z3s", b"sev_z3s [\xc8\xffa]"),
            // sev_w3S: MOVE = iv+s2v+vl ; PRE sev_w3S [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_w3S [\xc8\xffa]
            //   s2v: sev_w3S [\xc8\xffa]
            //   vl: sev_w3S [\xc8\xffa]
            ("sev_w3S", b"sev_w3S [\xc8\xffa]"),
            // sev_sc: MOVE = iv+s2v+vl ; PRE sev_sc [\xef\xbf\xbd\xef\xbf\xbdaA]
            //   iv: sev_sc [\xc8\xffaA]
            //   s2v: sev_sc [\xc8\xffaA]
            //   vl: sev_sc [\xc8\xffaA]
            ("sev_sc", b"sev_sc [\xc8\xffaA]"),
            // sev_s3s: MOVE = iv+s2v+vl ; PRE sev_s3s [ab\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_s3s [ab\xc8\xffa]
            //   s2v: sev_s3s [ab\xc8\xffa]
            //   vl: sev_s3s [ab\xc8\xffa]
            ("sev_s3s", b"sev_s3s [ab\xc8\xffa]"),
            // sev_u3s: MOVE = iv+s2v+vl ; PRE sev_u3s [ \xc3\x88a]
            //   iv: sev_u3s [\xc3\x88a]
            //   s2v: sev_u3s [\xc3\x88a]
            //   vl: sev_u3s [\xc3\x88a]
            ("sev_u3s", b"sev_u3s [\xc3\x88a]"),
            // sev_u4s: MOVE = iv+s2v+vl ; PRE sev_u4s [  \xc3\x88a]
            //   iv: sev_u4s [ \xc3\x88a]
            //   s2v: sev_u4s [ \xc3\x88a]
            //   vl: sev_u4s [ \xc3\x88a]
            ("sev_u4s", b"sev_u4s [ \xc3\x88a]"),
            // sev_um4: MOVE = iv+s2v+vl ; PRE sev_um4 [\xc3\x88a  ]
            //   iv: sev_um4 [\xc3\x88a ]
            //   s2v: sev_um4 [\xc3\x88a ]
            //   vl: sev_um4 [\xc3\x88a ]
            ("sev_um4", b"sev_um4 [\xc3\x88a ]"),
            // sev_e3s: MOVE = iv+s2v+vl ; PRE sev_e3s [  \xe2\x82\xac]
            //   iv: sev_e3s [\xe2\x82\xac]
            //   s2v: sev_e3s [\xe2\x82\xac]
            //   vl: sev_e3s [\xe2\x82\xac]
            ("sev_e3s", b"sev_e3s [\xe2\x82\xac]"),
            // sev_e5s: MOVE = iv+s2v+vl ; PRE sev_e5s [    \xe2\x82\xac]
            //   iv: sev_e5s [  \xe2\x82\xac]
            //   s2v: sev_e5s [  \xe2\x82\xac]
            //   vl: sev_e5s [  \xe2\x82\xac]
            ("sev_e5s", b"sev_e5s [  \xe2\x82\xac]"),
            // sev_p0s: MOVE = iv+s2v+vl ; PRE sev_p0s [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_p0s [\xc8\xffa]
            //   s2v: sev_p0s [\xc8\xffa]
            //   vl: sev_p0s [\xc8\xffa]
            ("sev_p0s", b"sev_p0s [\xc8\xffa]"),
            // sev_pS: MOVE = iv+s2v+vl ; PRE sev_pS [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_pS [\xc8\xffa]
            //   s2v: sev_pS [\xc8\xffa]
            //   vl: sev_pS [\xc8\xffa]
            ("sev_pS", b"sev_pS [\xc8\xffa]"),
            // sev_pms: MOVE = iv+s2v+vl ; PRE sev_pms [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_pms [\xc8\xffa]
            //   s2v: sev_pms [\xc8\xffa]
            //   vl: sev_pms [\xc8\xffa]
            ("sev_pms", b"sev_pms [\xc8\xffa]"),
            // sev_ps: MOVE = iv+s2v+vl ; PRE sev_ps [\xef\xbf\xbd\xef\xbf\xbda]
            //   iv: sev_ps [\xc8\xffa]
            //   s2v: sev_ps [\xc8\xffa]
            //   vl: sev_ps [\xc8\xffa]
            ("sev_ps", b"sev_ps [\xc8\xffa]"),
        ],
        &[],
    );
}

#[test]
fn d_d1_sev_stays() {
    check(
        &Cell {
            src: D_D1_SEV_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // sev_a5s: STAY = iv+s2v+vl ; PRE sev_a5s [   ab]
            //   iv: sev_a5s [   ab]
            //   s2v: sev_a5s [   ab]
            //   vl: sev_a5s [   ab]
            ("sev_a5s", b"sev_a5s [   ab]"),
            // sev_a2s: STAY = iv+s2v+vl ; PRE sev_a2s [ab]
            //   iv: sev_a2s [ab]
            //   s2v: sev_a2s [ab]
            //   vl: sev_a2s [ab]
            ("sev_a2s", b"sev_a2s [ab]"),
        ],
        &[],
    );
}

const D_D1_LIC_SRC: &str = r#"module top;
  string t_ab = "ab", t_hi = "\310\377a", s_u8 = "\303\210a", s_e3 = "\342\202\254", u;
  logic [7:0] b_a = 8'h41;
  function automatic string fl_w3s(input logic c); return c ? $sformatf("%3s", t_hi) : ""; endfunction
  function automatic string fl_w1s(input logic c); return c ? $sformatf("%1s", t_hi) : ""; endfunction
  function automatic string fl_m3s(input logic c); return c ? $sformatf("%-3s", t_hi) : ""; endfunction
  function automatic string fl_z3s(input logic c); return c ? $sformatf("%03s", t_hi) : ""; endfunction
  function automatic string fl_w3S(input logic c); return c ? $sformatf("%3S", t_hi) : ""; endfunction
  function automatic string fl_sc(input logic c); return c ? $sformatf("%s%c", t_hi, b_a) : ""; endfunction
  function automatic string fl_s3s(input logic c); return c ? $sformatf("%s%3s", t_ab, t_hi) : ""; endfunction
  function automatic string fl_u3s(input logic c); return c ? $sformatf("%3s", s_u8) : ""; endfunction
  function automatic string fl_u4s(input logic c); return c ? $sformatf("%4s", s_u8) : ""; endfunction
  function automatic string fl_um4(input logic c); return c ? $sformatf("%-4s", s_u8) : ""; endfunction
  function automatic string fl_e3s(input logic c); return c ? $sformatf("%3s", s_e3) : ""; endfunction
  function automatic string fl_e5s(input logic c); return c ? $sformatf("%5s", s_e3) : ""; endfunction
  function automatic string fl_p0s(input logic c); return c ? $sformatf("%0s", t_hi) : ""; endfunction
  function automatic string fl_pS(input logic c); return c ? $sformatf("%S", t_hi) : ""; endfunction
  function automatic string fl_pms(input logic c); return c ? $sformatf("%-s", t_hi) : ""; endfunction
  function automatic string fl_ps(input logic c); return c ? $sformatf("%s", t_hi) : ""; endfunction
  function automatic string fl_a5s(input logic c); return c ? $sformatf("%5s", t_ab) : ""; endfunction
  function automatic string fl_a2s(input logic c); return c ? $sformatf("%2s", t_ab) : ""; endfunction
  function automatic string fl_nst(input logic c); return c ? $sformatf("%s", $sformatf("%3s", t_hi)) : ""; endfunction
  function automatic string fl_cat(input logic c); return c ? {$sformatf("%3s", t_hi), "!"} : ""; endfunction
  task automatic show(input string tag); $display("%s %0d %h %h %h %h %h %h", tag, u.len(), u[0], u[1], u[2], u[3], u[4], u[5]); endtask
  initial begin
    u = fl_w3s(1); show("lic_w3s");
    u = fl_w1s(1); show("lic_w1s");
    u = fl_m3s(1); show("lic_m3s");
    u = fl_z3s(1); show("lic_z3s");
    u = fl_w3S(1); show("lic_w3S");
    u = fl_sc(1); show("lic_sc");
    u = fl_s3s(1); show("lic_s3s");
    u = fl_u3s(1); show("lic_u3s");
    u = fl_u4s(1); show("lic_u4s");
    u = fl_um4(1); show("lic_um4");
    u = fl_e3s(1); show("lic_e3s");
    u = fl_e5s(1); show("lic_e5s");
    u = fl_p0s(1); show("lic_p0s");
    u = fl_pS(1); show("lic_pS");
    u = fl_pms(1); show("lic_pms");
    u = fl_ps(1); show("lic_ps");
    u = fl_a5s(1); show("lic_a5s");
    u = fl_a2s(1); show("lic_a2s");
    u = fl_nst(1); show("lic_nst");
    u = fl_cat(1); show("lic_cat");
    #1 $finish;
  end
endmodule
"#;

#[test]
fn d_d1_lic_stays() {
    check(
        &Cell {
            src: D_D1_LIC_SRC,
            inputs: &[],
            plus: None,
            mode: Mode::Word,
            staged: true,
        },
        &[
            // lic_w3s: STAY = vl ; PRE lic_w3s 3 c8 ff 61 00 00 00
            //   vl: lic_w3s 3 c8 ff 61 00 00 00
            ("lic_w3s", b"lic_w3s 3 c8 ff 61 00 00 00"),
            // lic_w1s: STAY = vl ; PRE lic_w1s 3 c8 ff 61 00 00 00
            //   vl: lic_w1s 3 c8 ff 61 00 00 00
            ("lic_w1s", b"lic_w1s 3 c8 ff 61 00 00 00"),
            // lic_m3s: STAY = vl ; PRE lic_m3s 3 c8 ff 61 00 00 00
            //   vl: lic_m3s 3 c8 ff 61 00 00 00
            ("lic_m3s", b"lic_m3s 3 c8 ff 61 00 00 00"),
            // lic_z3s: STAY = vl ; PRE lic_z3s 3 c8 ff 61 00 00 00
            //   vl: lic_z3s 3 c8 ff 61 00 00 00
            ("lic_z3s", b"lic_z3s 3 c8 ff 61 00 00 00"),
            // lic_w3S: STAY = vl ; PRE lic_w3S 3 c8 ff 61 00 00 00
            //   vl: lic_w3S 3 c8 ff 61 00 00 00
            ("lic_w3S", b"lic_w3S 3 c8 ff 61 00 00 00"),
            // lic_sc: STAY = vl ; PRE lic_sc 4 c8 ff 61 41 00 00
            //   vl: lic_sc 4 c8 ff 61 41 00 00
            ("lic_sc", b"lic_sc 4 c8 ff 61 41 00 00"),
            // lic_s3s: STAY = vl ; PRE lic_s3s 5 61 62 c8 ff 61 00
            //   vl: lic_s3s 5 61 62 c8 ff 61 00
            ("lic_s3s", b"lic_s3s 5 61 62 c8 ff 61 00"),
            // lic_u3s: STAY = vl ; PRE lic_u3s 3 c3 88 61 00 00 00
            //   vl: lic_u3s 3 c3 88 61 00 00 00
            ("lic_u3s", b"lic_u3s 3 c3 88 61 00 00 00"),
            // lic_u4s: STAY = PRE (PLAN2 must-not-move: row 26 width drop) ; PRE lic_u4s 3 c3 88 61 00 00 00
            //   vl: lic_u4s 4 20 c3 88 61 00 00
            ("lic_u4s", b"lic_u4s 3 c3 88 61 00 00 00"),
            // lic_um4: STAY = PRE (PLAN2 must-not-move) ; PRE lic_um4 3 c3 88 61 00 00 00
            //   vl: lic_um4 4 c3 88 61 20 00 00
            ("lic_um4", b"lic_um4 3 c3 88 61 00 00 00"),
            // lic_e3s: STAY = vl ; PRE lic_e3s 3 e2 82 ac 00 00 00
            //   vl: lic_e3s 3 e2 82 ac 00 00 00
            ("lic_e3s", b"lic_e3s 3 e2 82 ac 00 00 00"),
            // lic_e5s: STAY = PRE (PLAN2 must-not-move) ; PRE lic_e5s 3 e2 82 ac 00 00 00
            //   vl: lic_e5s 5 20 20 e2 82 ac 00
            ("lic_e5s", b"lic_e5s 3 e2 82 ac 00 00 00"),
            // lic_p0s: STAY = vl ; PRE lic_p0s 3 c8 ff 61 00 00 00
            //   vl: lic_p0s 3 c8 ff 61 00 00 00
            ("lic_p0s", b"lic_p0s 3 c8 ff 61 00 00 00"),
            // lic_pS: STAY = vl ; PRE lic_pS 3 c8 ff 61 00 00 00
            //   vl: lic_pS 3 c8 ff 61 00 00 00
            ("lic_pS", b"lic_pS 3 c8 ff 61 00 00 00"),
            // lic_pms: STAY = vl ; PRE lic_pms 3 c8 ff 61 00 00 00
            //   vl: lic_pms 3 c8 ff 61 00 00 00
            ("lic_pms", b"lic_pms 3 c8 ff 61 00 00 00"),
            // lic_ps: STAY = vl ; PRE lic_ps 3 c8 ff 61 00 00 00
            //   vl: lic_ps 3 c8 ff 61 00 00 00
            ("lic_ps", b"lic_ps 3 c8 ff 61 00 00 00"),
            // lic_a5s: STAY = PRE (PLAN2 must-not-move) ; PRE lic_a5s 2 61 62 00 00 00 00
            //   vl: lic_a5s 5 20 20 20 61 62 00
            ("lic_a5s", b"lic_a5s 2 61 62 00 00 00 00"),
            // lic_a2s: STAY = vl ; PRE lic_a2s 2 61 62 00 00 00 00
            //   vl: lic_a2s 2 61 62 00 00 00 00
            ("lic_a2s", b"lic_a2s 2 61 62 00 00 00 00"),
            // lic_nst: STAY = vl ; PRE lic_nst 3 c8 ff 61 00 00 00
            //   vl: lic_nst 3 c8 ff 61 00 00 00
            ("lic_nst", b"lic_nst 3 c8 ff 61 00 00 00"),
            // lic_cat: STAY = vl ; PRE lic_cat 4 c8 ff 61 21 00 00
            //   vl: lic_cat 4 c8 ff 61 21 00 00
            ("lic_cat", b"lic_cat 4 c8 ff 61 21 00 00"),
        ],
        &[],
    );
}
