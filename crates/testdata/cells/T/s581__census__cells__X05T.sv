module top;
  generate
    case (1)
      4'b1x00 & 4'b0011: begin : g_a initial $display("X05T a"); end
      default: begin : g_def initial $display("X05T def"); end
    endcase
  endgenerate
endmodule
