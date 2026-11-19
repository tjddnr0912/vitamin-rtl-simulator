package p; let pf(logic [3:0] x) = x + 1'b1; endpackage
module t; import p::*;
  if (1) begin : g let gf(logic [3:0] x) = x + 1'b1; initial #1 $display("A g=%0d", gf(5'h1E)); end
  initial begin #2 $display("B p=%0d", pf(5'h1E)); end
  initial #100 $finish;
endmodule
