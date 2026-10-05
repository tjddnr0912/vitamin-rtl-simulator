import os
d = "/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s591/plan/a5"
PA = "package pa; localparam P = 3; endpackage\n"
PB = "package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage\n"
SAE = ("module sae #(parameter N = 0) ();\n"
       "  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction\n"
       "  localparam integer W = fae(N);\n  initial #3 $display(\"ae %m W=%0d\", W);\nendmodule\n")
SAC = ("module sacr #(parameter N = 0) (input logic [N:0] a);\n"
       "  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; 3: fc = 6; default: fc = 7; endcase endfunction\n"
       "  wire [7:0] r = {fc(N){1'b1}};\n  initial #3 $display(\"acr %m r=%b b=%0d\", r, $bits(a));\nendmodule\n")
IMPS = {"Inw": "  import pa::*;\n  import pb::P;\n", "Iwn": "  import pb::*;\n  import pa::P;\n",
        "Ipw": "  import pb::P;\n", "Ipn": "  import pa::P;\n",
        "Iaa": "  import pa::*;\n  import pb::*;\n"}
for n, imp in IMPS.items():
    hd = PA + PB + "module top;\n" + imp
    tl = "  initial #100 $finish;\nendmodule\n"
    open(f"{d}/{n}_cae.sv", "w").write(hd + "  sae #(.N(P)) u6 ();\n" + tl + SAE)
    open(f"{d}/{n}_kae.sv", "w").write(hd + "  localparam [64:0] KW = P;\n  sae #(.N(KW)) u6 ();\n" + tl + SAE)
    open(f"{d}/{n}_pae.sv", "w").write(hd + "  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction\n"
         "  localparam integer W = fae(P);\n  initial #3 $display(\"pae W=%0d\", W);\n" + tl)
    open(f"{d}/{n}_cacr.sv", "w").write(hd + "  sacr #(.N(P)) u8 (.a('0));\n" + tl + SAC)
open(f"{d}/Ewn.sv", "w").write("package pk; localparam E1 = 7; endpackage\nmodule top;\n  import pk::E1;\n  typedef enum {E0, E1} e_t;\n"
     "  localparam int K = E1;\n  initial #1 $display(\"ewn E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\nendmodule\n")
open(f"{d}/Iee_same.sv", "w").write(PA + "module top;\n  import pa::P;\n  import pa::P;\n  initial #1 $display(\"ees P=%0d\", P);\n  initial #100 $finish;\nendmodule\n")
open(f"{d}/Icue_nw.sv", "w").write(PA + PB + "import pa::P;\nmodule top;\n  import pb::P;\n  localparam int K = P;\n"
     "  initial #1 $display(\"cuenw P=%0d K=%0d b=%0d\", P, K, $bits(P));\n  initial #100 $finish;\nendmodule\n")
open(f"{d}/Icue_wn.sv", "w").write(PA + PB + "import pb::P;\nmodule top;\n  import pa::P;\n  localparam [64:0] K = P;\n"
     "  initial #1 $display(\"cuewn P=%0d K=%0d b=%0d\", P, K, $bits(P));\n  initial #100 $finish;\nendmodule\n")
open(f"{d}/Icuw_nw.sv", "w").write(PA + PB + "import pa::*;\nmodule top;\n  import pb::*;\n  localparam [64:0] K = P;\n"
     "  initial #1 $display(\"cuwnw P=%0d K=%0d\", P, K);\n  initial #100 $finish;\nendmodule\n")
open(f"{d}/Icuw_wn.sv", "w").write(PA + PB + "import pb::*;\nmodule top;\n  import pa::*;\n  localparam int K = P;\n"
     "  initial #1 $display(\"cuwwn P=%0d K=%0d b=%0d\", P, K, $bits(P));\n  initial #100 $finish;\nendmodule\n")
print(len([f for f in os.listdir(d) if f.endswith('.sv')]))
