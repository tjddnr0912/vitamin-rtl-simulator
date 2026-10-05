module top;
  generate
    case (4'd15 + 4'd1)
      4'd0: begin : g_a initial $display("W02 a"); end
      5'd16: begin : g_b initial $display("W02 b"); end
      default: begin : g_def initial $display("W02 def"); end
    endcase
  endgenerate
endmodule
