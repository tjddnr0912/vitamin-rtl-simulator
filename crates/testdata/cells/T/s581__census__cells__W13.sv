module top;
  generate
    case (4'sb1111)
      8'sb11111111: begin : g_a initial $display("W13 a"); end
      4'd0: begin : g_b initial $display("W13 b"); end
      default: begin : g_def initial $display("W13 def"); end
    endcase
  endgenerate
endmodule
