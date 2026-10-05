module top;
  generate
    case (2)
      1: begin : g_a initial $display("L01 a"); end
      2: begin : g_b initial $display("L01 b"); end
      default: begin : g_def initial $display("L01 def"); end
    endcase
  endgenerate
endmodule
