module top;
  generate
    case (4'd8)
      4'sb1000: begin : g_a initial $display("S23 a"); end
      default: begin : g_def initial $display("S23 def"); end
    endcase
  endgenerate
endmodule
