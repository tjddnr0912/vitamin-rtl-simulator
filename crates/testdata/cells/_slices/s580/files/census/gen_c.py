import os
D=os.path.join(os.path.dirname(os.path.abspath(__file__)),'c')
H="`timescale 1ns/1ns\n"
cells={}
def mod(body): return H+"module t;\n"+body+"  initial #100 $finish;\nendmodule\n"
cells["C01"]=mod("""  logic [3:0] v;
  initial begin v=4'b1100; if (v inside {4'b1?00}) $display("C01 then"); else $display("C01 else"); #1 $finish; end
""")
cells["C02"]=mod("""  logic [3:0] v;
  initial begin v=4'b1100; $display("C02 %h", (v inside {4'b1?00}) ? 8'd1 : 8'd2); #1 $finish; end
""")
cells["C03"]=mod("""  logic [3:0] v; wire w; assign w = v inside {4'b1?00};
  initial begin v=4'b1100; #1 $display("C03 %b", w); #1 $finish; end
""")
cells["C04"]=mod("""  logic [3:0] v; wire [7:0] w8; assign w8 = v inside {4'b1?00};
  initial begin v=4'b1100; #1 $display("C04 %b", w8); #1 $finish; end
""")
cells["C05"]=mod("""  logic [3:0] v;
  function automatic logic f(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  initial begin v=4'b1100; $display("C05 %b", f(v)); #1 $finish; end
""")
cells["C06"]=mod("""  logic [3:0] v;
  function logic f(input logic [3:0] a); f = a inside {4'b1?00}; endfunction
  initial begin v=4'b1100; $display("C06 %b", f(v)); #1 $finish; end
""")
cells["C07"]=mod("""  logic [3:0] v; logic r;
  task automatic tk(input logic [3:0] a, output logic o); o = a inside {4'b1?00}; endtask
  initial begin v=4'b1100; tk(v, r); $display("C07 %b", r); #1 $finish; end
""")
cells["C08"]=mod("""  logic [3:0] v; wire w;
  function automatic logic f(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  assign w = f(v);
  initial begin v=4'b1100; #1 $display("C08 %b", w); #1 $finish; end
""")
cells["C09"]=mod("""  function automatic logic cf(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  localparam logic L = cf(4'b1100);
  initial begin $display("C09 %b", L); #1 $finish; end
""")
cells["C10"]=mod("""  localparam logic L1 = (4'b1100 inside {4'b1?00});
  initial begin $display("C10 %b", L1); #1 $finish; end
""")
cells["C11"]=mod("""  localparam L2 = (4'b1100 inside {4'b1?00});
  initial begin $display("C11 %b", L2); #1 $finish; end
""")
cells["C12"]=mod("""  localparam logic [3:0] PV = 4'b1100; localparam L3 = PV inside {4'b1?00};
  initial begin $display("C12 %b", L3); #1 $finish; end
""")
cells["C13"]=mod("""  localparam logic [3:0] PV = 4'b1100;
  if (PV inside {4'b1?00}) begin : g initial $display("C13 then"); end
  else begin : g2 initial $display("C13 else"); end
  initial #1 $finish;
""")
cells["C14"]=mod("""  logic [3:0] v;
  initial begin v=4'b1100; case (1'b1) (v inside {4'b1?00}): $display("C14 item"); default: $display("C14 default"); endcase #1 $finish; end
""")
cells["C15"]=mod("""  logic [3:0] v;
  initial begin v=4'b0000; fork begin wait (v inside {4'b1?00}); $display("C15 woke %0t", $time); end join_none #1 v=4'b1000; #2 $display("C15 end"); $finish; end
""")
cells["C16"]=mod("""  logic [3:0] v;
  initial begin v=4'b1100; assert (v inside {4'b1?00}) $display("C16 pass"); else $display("C16 fail"); #1 $finish; end
""")
cells["C17"]=mod("""  logic [3:0] v; logic clk = 0; always #1 clk = ~clk;
  assert property (@(posedge clk) v inside {4'b1?00}) else $display("C17 fail %0t", $time);
  initial begin v=4'b1100; #6 $display("C17 end"); $finish; end
""")
cells["C18"]=H+"""class C; rand bit [3:0] x; constraint c { x inside {4'b1?00}; } endclass
module t;
  initial begin C o; int ok; o = new; for (int i=0;i<6;i++) begin ok = o.randomize(); $display("C18 %0d %b", ok, o.x); end #1 $finish; end
  initial #100 $finish;
endmodule
"""
cells["C19"]=H+"""class C; rand bit [3:0] x; endclass
module t;
  initial begin C o; int ok; o = new; for (int i=0;i<6;i++) begin ok = o.randomize() with { x inside {4'b1?00}; }; $display("C19 %0d %b", ok, o.x); end #1 $finish; end
  initial #100 $finish;
endmodule
"""
cells["C20"]=mod("""  logic [3:0] v; logic m;
  always_comb m = v inside {4'b1?00};
  initial begin v=4'b1100; #1 $display("C20 %b", m); #1 $finish; end
""")
cells["C21"]=mod("""  logic [3:0] v;
  initial begin v=4'b1000; while (v inside {4'b1?00}) v = v + 4'd1; $display("C21 %b", v); #1 $finish; end
""")
cells["C22"]=mod("""  logic [3:0] v; integer n;
  initial begin n=0; for (v=4'b1000; v inside {4'b1?0?}; v = v + 4'd1) n = n + 1; $display("C22 %0d", n); #1 $finish; end
""")
cells["C23"]=mod("""  logic [3:0] v; wire [3:0] y; assign y = (v inside {4'b1?00}) ? 4'd1 : 4'd2;
  initial begin v=4'b1100; #1 $display("C23 %h", y); #1 $finish; end
""")
cells["C24"]=mod("""  logic [3:0] v; logic clk = 0; always #1 clk = ~clk;
  assert property (@(posedge clk) (v inside {4'b1?00}) |-> ##1 (v inside {4'b1?00})) else $display("C24 fail %0t", $time);
  initial begin v=4'b1100; #6 $display("C24 end"); $finish; end
""")
cells["C25"]=H+"""package p; function automatic logic pf(input logic [3:0] a); return a inside {4'b1?00}; endfunction endpackage
module t;
  logic [3:0] v;
  initial begin v=4'b1100; $display("C25 %b", p::pf(v)); #1 $finish; end
  initial #100 $finish;
endmodule
"""
cells["C26"]=H+"""class K; function logic m(input logic [3:0] a); return a inside {4'b1?00}; endfunction endclass
module t;
  logic [3:0] v;
  initial begin K k; k = new; v=4'b1100; $display("C26 %b", k.m(v)); #1 $finish; end
  initial #100 $finish;
endmodule
"""
cells["C27"]=mod("""  localparam logic L4 = (4'bx100 inside {4'b1?00});
  initial begin $display("C27 %b", L4); #1 $finish; end
""")
cells["C28"]=mod("""  logic [(4'b1100 inside {4'b1?00}) : 0] w;
  initial begin $display("C28 %0d", $bits(w)); #1 $finish; end
""")
cells["C29"]=H+"""module m #(parameter logic P = 1'b0) (); initial $display("C29 %b", P); endmodule
module t;
  m #(.P(4'b1100 inside {4'b1?00})) u();
  initial #1 $finish;
  initial #100 $finish;
endmodule
"""
cells["C30"]=mod("""  logic [3:0] v;
  function automatic logic f(input logic [3:0] a); logic r; r = 1'b0; if (a inside {4'b1?00}) r = 1'b1; return r; endfunction
  localparam logic LC = f(4'b1000);
  initial begin v=4'b1100; $display("C30 %b %b", f(v), LC); #1 $finish; end
""")
cells["C31"]=mod("""  localparam logic [3:0] PV = 4'b1100;
  for (genvar i = 0; i < 2; i++) begin : g
    if ((PV + i) inside {4'b110?}) begin : h initial $display("C31 i=%0d in", i); end
    else begin : k initial $display("C31 i=%0d out", i); end
  end
  initial #1 $finish;
""")
cells["C32"]=mod("""  logic [3:0] v; logic [3:0] q [$];
  initial begin v=4'b1100; q.push_back(4'b1100); $display("C32 %b", q[0] inside {4'b1?00}); #1 $finish; end
""")
cells["C33"]=mod("""  logic [3:0] v; event e;
  initial begin v=4'b1100; @(v inside {4'b1?00}) $display("C33 edge"); end
  initial begin #1 v=4'b0100; #1 $display("C33 end"); $finish; end
""")
for k,s in cells.items(): open(os.path.join(D,k+".sv"),"w").write(s)
print(len(cells))
