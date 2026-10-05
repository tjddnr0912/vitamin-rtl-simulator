module top;
  generate
    case (1)
      4'b1x00 == 4'b1100: begin : g_a initial $display("X06 a"); end
      default: begin : g_def initial $display("X06 def"); end
    endcase
  endgenerate
endmodule
