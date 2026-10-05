package pk; localparam [64:0] KW = {1'b1, 64'd0}; endpackage
import pk::*;
module top;
  generate
    case (1)
      KW >> 64: begin : g_a initial $display("P05 a"); end
      default: begin : g_def initial $display("P05 def"); end
    endcase
  endgenerate
endmodule
