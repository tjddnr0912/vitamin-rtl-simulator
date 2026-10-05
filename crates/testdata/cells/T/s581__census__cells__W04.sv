module top;
  generate
    case (4'sb1111)
      -1: begin : g_a initial $display("W04 a"); end
      8'd0: begin : g_b initial $display("W04 b"); end
      default: begin : g_def initial $display("W04 def"); end
    endcase
  endgenerate
endmodule
