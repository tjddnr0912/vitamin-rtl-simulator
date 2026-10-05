module top;
  generate
    case (3)
      1: begin : g_a initial $display("M02 a"); end
      3: begin : g_b initial $display("M02 b"); end
      3: begin : g_c initial $display("M02 c"); end
      default: begin : g_def initial $display("M02 def"); end
    endcase
  endgenerate
endmodule
