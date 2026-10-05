module top;
  generate
    case (0)
      4'b1x00 & 4'b0011: begin : g_a initial $display("X05 a"); end
      default: begin : g_def initial $display("X05 def"); end
    endcase
  endgenerate
endmodule
