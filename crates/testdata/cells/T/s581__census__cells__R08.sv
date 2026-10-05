module top;
  localparam M = 1;
  generate
    case (M)
      $isunknown(4'bx100 ==? 4'b1?00): begin : g_a initial $display("R08 a"); end
      1: begin : g_b initial $display("R08 b"); end
      default: begin : g_def initial $display("R08 def"); end
    endcase
  endgenerate
endmodule
