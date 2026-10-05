module top;
  generate
    casez (1)
      1: begin : g_a initial $display("Z01 a"); end
      default: begin : g_def initial $display("Z01 def"); end
    endcase
  endgenerate
endmodule
