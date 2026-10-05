module top;
  generate
    case (-1)
      33'h0FFFFFFFF: begin : g_a initial $display("S27 a"); end
      default: begin : g_def initial $display("S27 def"); end
    endcase
  endgenerate
endmodule
