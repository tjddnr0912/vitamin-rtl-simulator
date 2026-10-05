module top;
  generate
    case (2)
      1, 2: begin : g_a initial $display("M01 a"); end
      2: begin : g_b initial $display("M01 b"); end
      default: begin : g_def initial $display("M01 def"); end
    endcase
  endgenerate
endmodule
