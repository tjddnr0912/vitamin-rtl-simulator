module top;
  generate
    case (65'd1)
      1: begin : g_a initial $display("S02 a"); end
      default: begin : g_def initial $display("S02 def"); end
    endcase
  endgenerate
endmodule
