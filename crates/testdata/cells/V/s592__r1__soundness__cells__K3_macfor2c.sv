`define F2(n1, n2) for (genvar i = 0; i < n1; i++) begin : f1 initial #1 $display("@%m"); end for (genvar j = 0; j < n2; j++) begin : f2 initial #1 $display("@%m"); end
module top;
  `F2(2, 3)
  initial #5 $finish;
endmodule
