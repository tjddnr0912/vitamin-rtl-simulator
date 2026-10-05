module top;
  int y;
  initial begin unique case (y) 0: y = 1; default: y = 2; endcase end
`include "does_not_exist.svh"
endmodule
