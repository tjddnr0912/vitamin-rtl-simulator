module top;
  generate
    case (4'd8 << 1)
      5'd16: begin : g_a initial $display("W18 a"); end
      4'd0: begin : g_b initial $display("W18 b"); end
      default: begin : g_def initial $display("W18 def"); end
    endcase
  endgenerate
endmodule
