module top;
  generate
    case (4'sb1000)
      -8: begin : g_a initial $display("S20 a"); end
      default: begin : g_def initial $display("S20 def"); end
    endcase
  endgenerate
endmodule
