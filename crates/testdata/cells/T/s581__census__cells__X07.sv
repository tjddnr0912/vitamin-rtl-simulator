module top;
  generate
    case (0)
      4'b1x00 == 4'b0000: begin : g_a initial $display("X07 a"); end
      default: begin : g_def initial $display("X07 def"); end
    endcase
  endgenerate
endmodule
