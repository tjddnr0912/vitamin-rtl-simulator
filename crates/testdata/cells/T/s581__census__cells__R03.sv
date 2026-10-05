module top;
  localparam M = 1;
  generate
    case (M)
      0: begin : g_a initial $display("R03 a"); end
      1: begin : g_b initial $display("R03 b"); end
      default: begin : g_def initial $display("R03 def"); end
    endcase
  endgenerate
endmodule
