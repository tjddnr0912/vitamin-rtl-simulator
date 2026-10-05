module top;
  generate
    case (4'b1100)
      4'b1x00: begin : g_a initial $display("X01 a"); end
      default: begin : g_def initial $display("X01 def"); end
    endcase
  endgenerate
endmodule
