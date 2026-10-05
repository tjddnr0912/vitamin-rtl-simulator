module top;
  generate
    casex (1)
      1: begin : g_a initial $display("Z02 a"); end
      default: begin : g_def initial $display("Z02 def"); end
    endcase
  endgenerate
endmodule
