module top;
  if (1) begin : g
    case (8'd2)
      K: begin : a initial #1 $display("@inner"); end
      default: begin : b initial #1 $display("@def"); end
    endcase
`include "DM10c_inc.svh"
  end
  initial #5 $finish;
endmodule
