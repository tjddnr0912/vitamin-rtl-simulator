module top;
  generate
    case (1)
      1: begin : g_a initial $display("M05 a"); end
      $isunknown(4'bx100 ==? 4'b1?00): begin : g_b initial $display("M05 b"); end
      default: begin : g_def initial $display("M05 def"); end
    endcase
  endgenerate
endmodule
