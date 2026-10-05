module top;
  generate
    case (8'sb11111111)
      4'sb1111: begin : g_a initial $display("S12 a"); end
      default: begin : g_def initial $display("S12 def"); end
    endcase
  endgenerate
endmodule
