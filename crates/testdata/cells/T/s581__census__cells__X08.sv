module top;
  generate
    case (1)
      4'b1x00 === 4'b1x00: begin : g_a initial $display("X08 a"); end
      default: begin : g_def initial $display("X08 def"); end
    endcase
  endgenerate
endmodule
