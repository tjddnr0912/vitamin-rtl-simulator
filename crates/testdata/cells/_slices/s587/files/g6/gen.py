import os
D = os.path.dirname(os.path.abspath(__file__))
F = """  function automatic int f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction"""
F4 = """  function automatic logic [3:0] f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction"""
G8 = """  function automatic logic [7:0] g(input int a);
    g = "A";
    @Q@if (a == 1) g = "B";
  endfunction"""
L = {}
L['g01a_hdr_sub'] = """module sub #(parameter int W = f(2)) ();
@F@
  initial begin #1 $display("W=%0d", W); $finish; end
endmodule
module top;
  sub u();
endmodule
"""
L['g01b_hdr_top'] = """module top #(parameter int P = f(2)) ();
@F@
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['g02a_ifc_hdr'] = """interface ifc #(parameter int P = f(2)) ();
@F@
endinterface
module top;
  ifc i();
  initial begin #1 $display("P=%0d", i.P); $finish; end
endmodule
"""
L['g02b_ifc_body'] = """interface ifc ();
@F@
  parameter int P = f(2);
endinterface
module top;
  ifc i();
  initial begin #1 $display("P=%0d", i.P); $finish; end
endmodule
"""
L['g02c_ifc_real'] = """interface ifc ();
@F@
  parameter real R = f(2);
endinterface
module top;
  ifc i();
  initial begin #1 $display("R=%0.2f", i.R); $finish; end
endmodule
"""
L['g02d_ifc_lp'] = """interface ifc ();
@F@
  localparam int P = f(2);
endinterface
module top;
  ifc i();
  initial begin #1 $display("P=%0d", i.P); $finish; end
endmodule
"""
L['g03a_untyped'] = """module top;
@F4@
  localparam P = f(2);
  initial begin #1 $display("P=%0d b=%0d", P, $bits(P)); $finish; end
endmodule
"""
L['g03b_typed8'] = """module top;
@F@
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("P=%h b=%0d", P, $bits(P)); $finish; end
endmodule
"""
L['g03c_real'] = """module top;
@F@
  localparam real R = f(2);
  initial begin #1 $display("R=%0.2f", R); $finish; end
endmodule
"""
L['g04_gen_enum'] = """module top;
@F@
  if (1) begin : g
    typedef enum int {A = f(2), B} e_t;
    initial begin #1 $display("A=%0d B=%0d", A, B); $finish; end
  end
endmodule
"""
L['g05a_inst_arr'] = """module sub(input logic [3:0] p);
  initial #1 if (p == 4'h7) $display("hit %m");
endmodule
module top;
@F@
  logic [31:0] w = 32'h76543210;
  sub u [f(2):0] (.p(w));
  initial #2 $finish;
endmodule
"""
L['g05b_inst_child'] = """module sub(input logic [f(2)-4:0] p);
@F@
  initial #1 if (p == 4'h7) $display("hit %m b=%0d", $bits(p));
endmodule
module top;
  logic [31:0] w = 32'h76543210;
  sub u [7:0] (.p(w));
  initial #2 $finish;
endmodule
"""
L['g05c_formal_arr'] = """module top;
@F@
  function automatic int sz(input int a [0:f(2)]);
    return $size(a) * 100 + a[7];
  endfunction
  int arr [0:7];
  initial begin
    foreach (arr[i]) arr[i] = i + 10;
    #1 $display("s=%0d", sz(arr)); $finish;
  end
endmodule
"""
L['g05d_str_arr'] = """module top;
@F@
  string s [0:f(2)] = '{"a","b","c","d","e","f","g","h"};
  initial begin #1 $display("n=%0d s7=%s", $size(s), s[7]); $finish; end
endmodule
"""
L['g05e_lp_range'] = """module top;
@F@
  localparam logic [f(2):0] P = '1;
  initial begin #1 $display("P=%h b=%0d", P, $bits(P)); $finish; end
endmodule
"""
L['g05f_enum_base'] = """module top;
@F@
  typedef enum logic [f(2):0] {A = 8'hff, B = 0} e_t;
  e_t e = A;
  initial begin #1 $display("b=%0d e=%h", $bits(e), e); $finish; end
endmodule
"""
L['g05g_vcd'] = """module top;
@F@
  logic [f(2):0] pk = 8'h5a;
  logic [0:f(2)] pa = 8'h5a;
  logic [f(2):-2] pn = 10'h15a;
  initial begin
    $dumpfile("@VCD@");
    $dumpvars(0, top);
    #1 pk = 8'ha5; pa = 8'ha5; pn = 10'h2a5;
    #1 $display("done"); $finish;
  end
endmodule
"""
L['g05h_neg_lsb'] = """module top;
@F@
  logic [f(2):-2] v = '1;
  initial begin #1 v[-2] = 1'b0; $display("b=%0d l=%0d r=%0d v=%h", $bits(v), $left(v), $right(v), v); $finish; end
endmodule
"""
L['g05i_asc_lsb'] = """module top;
@F@
  logic [0:f(2)] v = 8'h0f;
  initial begin #1 $display("b=%0d l=%0d r=%0d v0=%b v7=%b", $bits(v), $left(v), $right(v), v[0], v[7]); $finish; end
endmodule
"""
L['g05j_lp_arr'] = """module top;
@F@
  localparam int A [0:f(2)] = '{10,11,12,13,14,15,16,17};
  localparam int Q = A[7];
  initial begin #1 $display("n=%0d a7=%0d q=%0d", $size(A), A[7], Q); $finish; end
endmodule
"""
L['g05k_task_local'] = """module top;
@F@
  task automatic t;
    logic [f(2):0] x;
    x = '1;
    $display("tb=%0d x=%h", $bits(x), x);
  endtask
  initial begin #1 t(); $finish; end
endmodule
"""
L['g05l_task_susp'] = """module top;
@F@
  task automatic t;
    logic [f(2):0] x;
    x = '1;
    #1 $display("tb=%0d x=%h", $bits(x), x);
  endtask
  initial begin #1 t(); $finish; end
endmodule
"""
L['g05m_port_unp'] = """module sub(input logic p [f(2):0]);
@F@
  initial #1 $display("l=%0d r=%0d i=%0d p7=%b p0=%b", $left(p), $right(p), $increment(p), p[7], p[0]);
endmodule
module top;
  logic arr [7:0];
  initial begin foreach (arr[i]) arr[i] = 1'b0; arr[7] = 1'b1; end
  sub u(.p(arr));
  initial #2 $finish;
endmodule
"""
L['g05n_net_unp'] = """module top;
@F@
  logic u [f(2):0];
  initial begin
    u = '{1'b1,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0};
    #1 $display("l=%0d r=%0d i=%0d u7=%b u0=%b", $left(u), $right(u), $increment(u), u[7], u[0]); $finish;
  end
endmodule
"""
L['g05o_task_unp'] = """module top;
@F@
  task automatic t;
    logic x [f(2):0];
    x = '{1'b1,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0};
    $display("x7=%b x0=%b i=%0d", x[7], x[0], $increment(x));
  endtask
  initial begin #1 t(); $finish; end
endmodule
"""
L['g05p_md_packed'] = """module top;
@F@
  logic [f(2)-6:0][3:0] m2 = 8'hc3;
  logic [1:0][f(2)-4:0] m3 = 8'h3c;
  initial begin #1 $display("b2=%0d b3=%0d m2=%h m3=%h e=%h", $bits(m2), $bits(m3), m2[1], m3[0], m2); $finish; end
endmodule
"""
L['g05q_bits_prescan'] = """module top;
@F@
  localparam int B = $bits(late);
  logic [f(2):0] late;
  initial begin #1 $display("B=%0d", B); $finish; end
endmodule
"""
L['g06a_ur_bits'] = """module top;
@F@
  logic [3:0] u [f(2)];
  initial begin #1 $display("b=%0d", $bits(u)); $finish; end
endmodule
"""
L['g06b_ur_query'] = """module top;
@F@
  logic [3:0] u [f(2)];
  initial begin #1 $display("l=%0d r=%0d i=%0d s=%0d d=%0d", $left(u), $right(u), $increment(u), $size(u), $dimensions(u)); $finish; end
endmodule
"""
L['g06c_str_n'] = """module top;
@F@
  string s [f(2)];
  string s2 [f(2)] = '{"a","b","c","d","e","f","g"};
  initial begin #1 $display("n=%0d n2=%0d s6=%s", $size(s), $size(s2), s2[6]); $finish; end
endmodule
"""
L['g06d_formal_n'] = """module top;
@F@
  function automatic int sz(input int a [f(2)]);
    return $size(a) * 100 + a[6];
  endfunction
  int arr [7];
  initial begin
    foreach (arr[i]) arr[i] = i + 10;
    #1 $display("s=%0d", sz(arr)); $finish;
  end
endmodule
"""
L['g06e_blk_str'] = """module top;
@F@
  initial begin : b
    string s [f(2)] = '{"a","b","c","d","e","f","g"};
    #1 $display("n=%0d s6=%s", $size(s), s[6]); $finish;
  end
endmodule
"""
L['g06f_frame_str'] = """module top;
@F@
  task automatic t;
    string s [f(2)];
    s[6] = "z";
    #1 $display("n=%0d s6=%s", $size(s), s[6]);
  endtask
  initial begin #1 t(); $finish; end
endmodule
"""
L['g07a_cast_lp'] = """module top;
@F@
  localparam int P = f(2)'(9'h1FF);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['g07b_cast_bits'] = """module top;
@F@
  logic [15:0] x = 16'hFFFF;
  initial begin #1 $display("b=%0d", $bits(f(2)'(x))); $finish; end
endmodule
"""
L['g07c_cast_rpt'] = """module top;
@F@
  int n;
  initial begin n = 0; #1 repeat (f(2)'(4'hF)) n++; $display("n=%0d", n); $finish; end
endmodule
"""
DUT = """module dut;
  logic [15:0] v = 16'hABCD;
endmodule
"""
L['g08a_hier_psr'] = DUT + """module top;
@F@
  dut d();
  initial begin #1 $display("r=%h", d.v[0 +: f(2)]); $finish; end
endmodule
"""
L['g08b_hier_psw'] = DUT + """module top;
@F@
  dut d();
  initial begin #1 d.v[0 +: f(2)] = '0; #1 $display("v=%h", d.v); $finish; end
endmodule
"""
L['g08c_hier_prr'] = DUT + """module top;
@F@
  dut d();
  initial begin #1 $display("r=%h", d.v[f(2):0]); $finish; end
endmodule
"""
L['g08d_hier_prw'] = DUT + """module top;
@F@
  dut d();
  initial begin #1 d.v[f(2):0] = '0; #1 $display("v=%h", d.v); $finish; end
endmodule
"""
L['g09a_reg_rep'] = """module top;
@F@
  logic [7:0] r;
  initial begin #1 r = ({f(2){1'b1}} + 7'h01) >> 1; $display("r=%h", r); $finish; end
endmodule
"""
L['g09b_reg_ps'] = """module top;
@F@
  logic [15:0] v = 16'hFFFF;
  logic [7:0] r;
  initial begin #1 r = (v[0 +: f(2)] + 7'h01) >> 1; $display("r=%h", r); $finish; end
endmodule
"""
L['g09c_reg_pr'] = """module top;
@F@
  logic [15:0] v = 16'hFFFF;
  logic [7:0] r;
  initial begin #1 r = (v[f(2)-1:0] + 7'h01) >> 1; $display("r=%h", r); $finish; end
endmodule
"""
L['g09d_reg_hpr'] = """module dut;
  logic [15:0] v = 16'hFFFF;
endmodule
module top;
@F@
  dut d();
  logic [7:0] r;
  initial begin #1 r = (d.v[f(2)-1:0] + 7'h01) >> 1; $display("r=%h", r); $finish; end
endmodule
"""
L['g09e_reg_ca'] = """module top;
@F@
  logic [7:0] r;
  assign r = ({f(2){1'b1}} + 7'h01) >> 1;
  initial begin #1 $display("r=%h", r); $finish; end
endmodule
"""
L['g10a_neg_cnt'] = """module top;
@F@
  logic [7:0] r;
  initial begin #1 r = {f(2)-8{1'b1}}; $display("r=%h", r); $finish; end
endmodule
"""
L['g10b_zero_cnt'] = """module top;
@F@
  logic [7:0] r;
  initial begin #1 r = {{f(2)-7{1'b1}}, 4'h5}; $display("r=%h", r); $finish; end
endmodule
"""
L['g11a_md_pr'] = """module top;
@F@
  logic [15:0][3:0] m = 64'h0123456789ABCDEF;
  initial begin #1 $display("m=%h", m[f(2):0]); $finish; end
endmodule
"""
L['g11b_md3'] = """module top;
@F@
  logic [3:0][7:0][3:0] m3 = 128'h0123456789ABCDEF_FEDCBA9876543210;
  initial begin #1 $display("m=%h", m3[1][f(2):0]); $finish; end
endmodule
"""
L['g11c_md_prw'] = """module top;
@F@
  logic [15:0][3:0] m = 64'h0123456789ABCDEF;
  initial begin #1 m[f(2):0] = '0; $display("m=%h", m); $finish; end
endmodule
"""
L['g11d_md3w'] = """module top;
@F@
  logic [3:0][7:0][3:0] m3 = 128'h0123456789ABCDEF_FEDCBA9876543210;
  initial begin #1 m3[1][f(2):0] = '0; $display("m=%h", m3); $finish; end
endmodule
"""
L['g12a_elab_s'] = """module top;
@G8@
  $info("s=%s", g(2));
  initial begin #1 $finish; end
endmodule
"""
L['g12b_ifc_elab'] = """interface ifc ();
@F@
  $info("el=%0d", f(2));
endinterface
module top;
  ifc i();
  initial begin #1 $finish; end
endmodule
"""
L['g12c_fatal1'] = """module top;
@F@
  $fatal(f(2)-7, "boom");
  initial begin #1 $finish; end
endmodule
"""
L['g12d_elab_mod'] = """module top;
@F@
  $info("el=%0d", f(2));
  initial begin #1 $finish; end
endmodule
"""
L['g13a_lp_rpt'] = """module top;
@F@
  localparam int P = f(2);
  int n;
  initial begin n = 0; #1 repeat (f(2)) n++; $display("P=%0d n=%0d", P, n); $finish; end
endmodule
"""
L['g13b_rpt_lp'] = """module top;
@F@
  int n;
  initial begin n = 0; #1 repeat (f(2)) n++; $display("P=%0d n=%0d", P, n); $finish; end
  localparam int P = f(2);
endmodule
"""
for name, t in L.items():
    for suf, q in (('u', 'unique '), ('p', '')):
        b = f"{name}_{suf}"
        txt = t.replace('@F@', F).replace('@F4@', F4).replace('@G8@', G8).replace('@Q@', q).replace('@VCD@', b + '.vcd')
        open(os.path.join(D, b + '.sv'), 'w').write(txt)
print(len(L), 'lanes', 2 * len(L), 'files')
L2 = {}
L2['g05r_str_arr2'] = """module top;
@F@
  string s [0:f(2)] = '{"a","b","c","d","e","f","g","h"};
  initial begin #1 $display("s7=%s s0=%s", s[7], s[0]); $finish; end
endmodule
"""
L2['g05s_str_arr3'] = """module top;
@F@
  string s [0:f(2)];
  initial begin s[7] = "z"; #1 $display("s7=%s", s[7]); $finish; end
endmodule
"""
L2['g05t_task_unp2'] = """module top;
@F@
  task automatic t;
    logic x [f(2):0];
    foreach (x[i]) x[i] = 1'b0;
    x[7] = 1'b1;
    $display("x7=%b x0=%b l=%0d i=%0d", x[7], x[0], $left(x), $increment(x));
  endtask
  initial begin #1 t(); $finish; end
endmodule
"""
L2['g05u_bits_prescan2'] = """module top;
@F@
  logic [f(2):0] early;
  localparam int B = $bits(early);
  initial begin #1 $display("B=%0d", B); $finish; end
endmodule
"""
L2['g06g_str_n2'] = """module top;
@F@
  string s2 [f(2)] = '{"a","b","c","d","e","f","g"};
  initial begin #1 $display("s6=%s s0=%s", s2[6], s2[0]); $finish; end
endmodule
"""
L2['g06h_str_n3'] = """module top;
@F@
  string s [f(2)];
  initial begin s[6] = "z"; #1 $display("s6=%s", s[6]); $finish; end
endmodule
"""
L2['g06i_blk_str2'] = """module top;
@F@
  initial begin : b
    string s [f(2)] = '{"a","b","c","d","e","f","g"};
    #1 $display("s6=%s s0=%s", s[6], s[0]); $finish;
  end
endmodule
"""
L2['g06j_ur_bits_param'] = """module top;
@F@
  logic [3:0] u [f(2)];
  localparam int B = $bits(u);
  initial begin #1 $display("B=%0d", B); $finish; end
endmodule
"""
L2['g04b_gen_enum2'] = """module top;
@F@
  if (1) begin : g
    typedef enum int {A = f(2), B} e_t;
    e_t e = B;
    initial begin #1 $display("e=%0d", e); $finish; end
  end
endmodule
"""
L2['g12e_elab_s2'] = """module top;
@F@
  $info("s=%s", "x", f(2));
  initial begin #1 $finish; end
endmodule
"""
import sys
if len(sys.argv) > 1 and sys.argv[1] == 'l2':
    for name, t in L2.items():
        for suf, q in (('u', 'unique '), ('p', '')):
            b = f"{name}_{suf}"
            txt = t.replace('@F@', F).replace('@F4@', F4).replace('@G8@', G8).replace('@Q@', q).replace('@VCD@', b + '.vcd')
            open(os.path.join(D, b + '.sv'), 'w').write(txt)
    print(len(L2), 'L2 lanes')
