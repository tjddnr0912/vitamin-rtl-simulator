primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  logic k = 1'b0;
  wire o1, o2;
  logic m;
  logic t, clk, q;
  inv u1(o1, k);
  always_comb m = ~o1;
  inv u2(o2, m);
  always @(t) $display("b2 t=%0t o1=%b m=%b o2=%b", $time, o1, m, o2);
  always @(posedge clk) q <= o2;
  initial begin t = 1'b1; clk = 1'b1; #1 $display("e t=%0t o1=%b m=%b o2=%b q=%b", $time, o1, m, o2, q); $finish; end
endmodule
