package pk;
  logic a = 0, b = 0;
  initial begin unique if (a) $display("a"); else if (b) $display("b"); end
endpackage
module top;
  initial #1 $finish;
endmodule
