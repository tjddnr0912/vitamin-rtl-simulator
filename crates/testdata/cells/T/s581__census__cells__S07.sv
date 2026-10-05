module top;
  generate
    case (4'sb1111)
      4'b1111: begin : g_a initial $display("S07 a"); end
      default: begin : g_def initial $display("S07 def"); end
    endcase
  endgenerate
endmodule
