`timescale 1ns/1ns
class C; rand bit [3:0] x; endclass
module t;
  initial begin C o; int ok; o = new; for (int i=0;i<6;i++) begin ok = o.randomize() with { x inside {4'b1?00}; }; $display("C19 %0d %b", ok, o.x); end #1 $finish; end
  initial #100 $finish;
endmodule
