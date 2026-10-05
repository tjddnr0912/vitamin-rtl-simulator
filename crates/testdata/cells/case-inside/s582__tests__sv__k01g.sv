`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  localparam P = 2;
  generate case (P) inside 1: begin : g1 initial $display("one"); end [2:3]: begin : g2 initial $display("two"); end endcase endgenerate
  initial #1 $finish;
endmodule
