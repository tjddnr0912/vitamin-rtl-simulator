module top;
  generate
    case (4'd8 << 1)
      4'd0: begin : g_a initial $display("W19 a"); end
      default: begin : g_def initial $display("W19 def"); end
    endcase
  endgenerate
endmodule
