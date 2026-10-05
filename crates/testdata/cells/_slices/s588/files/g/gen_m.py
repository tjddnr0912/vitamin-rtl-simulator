import os
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s588/g/m'
FD='''  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
'''
def mod(name, body, disp, pre='', post=''):
    open(os.path.join(S,name+'.sv'),'w').write(pre+'module top;\n'+body+'  initial begin #2 $display('+disp+'); $finish; end\n  initial #100 $finish;\nendmodule\n'+post)
mod('m01_lp4', FD+"  localparam logic [3:0] P = fd(2);\n", '"P=%0d", P')
mod('m02_untyped', FD+"  parameter P = fd(2);\n", '"P=%0d b=%0d", P, $bits(P)')
mod('m03_lpint', FD+"  localparam int P = fd(2);\n", '"P=%0d", P')
mod('m04_packed_bound', FD+"  logic [fd(2):0] v;\n", '"b=%0d", $bits(v)')
mod('m05_gen_if', FD+"  if (fd(2) == 2) begin : g1 initial $display(\"T\"); end else begin : g2 initial $display(\"E\"); end\n", '"done"')
mod('m06_repl', FD+"  localparam logic [7:0] P = {fd(2){1'b1}};\n", '"P=%b", P')
mod('m07_ipsel_width', FD+"  localparam logic [7:0] V = 8'hA5;\n", '"r=%b", V[0 +: fd(2)]')
mod('m08_override', FD+"  child #(.P(fd(2))) u();\n", '"done"', post='module child #(parameter logic [3:0] P = 4\'d9) ();\n  initial #1 $display("P=%0d", P);\nendmodule\n')
mod('m09_hdr_default', "  child u();\n", '"done"', post='module child #(parameter logic [3:0] P = fd(2)) ();\n'+FD+'  initial #1 $display("P=%0d", P);\nendmodule\n')
mod('m10_pkg_scoped', "  localparam logic [3:0] P = pk::fd(2);\n", '"P=%0d", P', pre='package pk;\n'+FD+'endpackage\n')
mod('m11_pkg_import', "  import pk::*;\n  localparam logic [3:0] P = fd(2);\n", '"P=%0d", P', pre='package pk;\n'+FD+'endpackage\n')
mod('m12_unit_fn', "  localparam logic [3:0] P = fd(2);\n", '"P=%0d", P', pre=FD)
mod('m14_nested', FD+"  function automatic logic [3:0] g(input int a);\n    g = fd(a) + 4'd1;\n  endfunction\n  localparam logic [3:0] P = g(2);\n", '"P=%0d", P')
mod('m15_enum', FD+"  typedef enum logic [3:0] {A = fd(2), B} e_t;\n", '"A=%0d B=%0d", A, B')
mod('m16_gen_case', FD+"  case (fd(2)) 4'd1: begin : g1 initial $display(\"C1\"); end 4'd2: begin : g2 initial $display(\"C2\"); end default: begin : gd initial $display(\"CD\"); end endcase\n", '"done"')
mod('m17_gen_for', FD+"  for (genvar i = 0; i < fd(2); i = i + 1) begin : g initial $display(\"I%0d\", i); end\n", '"done"')
mod('m18_inst_array', "  child u[1:0] ();\n", '"done"', post='module child #(parameter logic [3:0] P = fd(2)) ();\n'+FD+'  initial #1 $display("%m P=%0d", P);\nendmodule\n')
mod('m19_repeat', FD+"  int n;\n  initial begin n = 0; repeat (fd(2)) n = n + 1; $display(\"n=%0d\", n); end\n", '"done"')
mod('m20_delay', FD+"  initial begin #(fd(2)) $display(\"t=%0t\", $time); end\n", '"done"')
mod('m21_gen_lp', FD+"  if (1) begin : g localparam logic [3:0] P = fd(2); end\n", '"P=%0d", g.P')
mod('m22_unpacked', FD+"  logic [7:0] m [fd(2)];\n", '"s=%0d", $size(m)')
mod('m23_cast_size', FD+"  localparam logic [7:0] P = fd(2)'(8'hFF);\n", '"P=%b", P')
mod('m24_case_label_rt', FD+"  initial begin case (4'd2) fd(2): $display(\"M\"); default: $display(\"D\"); endcase end\n", '"done"')
mod('m25_rt_call', FD+"  initial begin #1 $display(\"r=%0d\", fd(2)); end\n", '"done"')
