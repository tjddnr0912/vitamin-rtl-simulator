module top;
  generate
    case (1)
      $isunknown(4'bx100 ==? 4'b1?00): begin : g_a initial $display("M04 a"); end
      1: begin : g_b initial $display("M04 b"); end
      default: begin : g_def initial $display("M04 def"); end
    endcase
  endgenerate
endmodule
