module top;
  generate
    case (-33'sd1)
      -1: begin : g_a initial $display("S18 a"); end
      default: begin : g_def initial $display("S18 def"); end
    endcase
  endgenerate
endmodule
