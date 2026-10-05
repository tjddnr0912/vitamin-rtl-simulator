`timescale 1ns/1ns
class C; rand bit [3:0] x; constraint c { x inside {4'b1?00}; } endclass
module t;
  initial begin C o; int ok; o = new; for (int i=0;i<6;i++) begin ok = o.randomize(); $display("C18 %0d %b", ok, o.x); end #1 $finish; end
  initial #100 $finish;
endmodule
