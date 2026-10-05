module top;
  generate
    case (4'sb1000)
      8'sb11111000: begin : g_a initial $display("W07 a"); end
      4'd0: begin : g_b initial $display("W07 b"); end
      default: begin : g_def initial $display("W07 def"); end
    endcase
  endgenerate
endmodule
