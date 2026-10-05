`define GF(lbl, n) for (genvar i = 0; i < n; i++) begin : lbl initial #1 $display("@%m"); end
module top;
  `GF(f1, 2)
  `GF(f2, 3)
  initial #5 $finish;
endmodule
