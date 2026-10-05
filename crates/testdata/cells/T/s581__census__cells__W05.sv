module top;
  generate
    case (4'sb1111)
      -1: begin : g_a initial $display("W05 a"); end
      8'sd0: begin : g_b initial $display("W05 b"); end
      default: begin : g_def initial $display("W05 def"); end
    endcase
  endgenerate
endmodule
