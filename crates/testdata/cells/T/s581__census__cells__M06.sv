module top;
  generate
    case (0)
      $isunknown(4'bx100 ==? 4'b1?00): begin : g_a initial $display("M06 a"); end
      0: begin : g_b initial $display("M06 b"); end
      default: begin : g_def initial $display("M06 def"); end
    endcase
  endgenerate
endmodule
