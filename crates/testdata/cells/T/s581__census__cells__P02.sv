package pk; localparam logic signed [3:0] KS = -1; endpackage
import pk::*;
module top;
  generate
    case (4'b1111)
      KS: begin : g_a initial $display("P02 a"); end
      default: begin : g_def initial $display("P02 def"); end
    endcase
  endgenerate
endmodule
