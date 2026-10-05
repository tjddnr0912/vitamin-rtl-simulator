module top;
  generate
    case (-64'sd1)
      -65'sd1: begin : g_a initial $display("S35 a"); end
      default: begin : g_def initial $display("S35 def"); end
    endcase
  endgenerate
endmodule
