module top;
  generate
    case (4'sb1111)
      4'sb1111: begin : g_a initial $display("W06 a"); end
      8'd0: begin : g_b initial $display("W06 b"); end
      default: begin : g_def initial $display("W06 def"); end
    endcase
  endgenerate
endmodule
