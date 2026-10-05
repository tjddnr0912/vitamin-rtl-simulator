module top;
  generate
    case (-1)
      33'h1FFFFFFFF: begin : g_a initial $display("S26 a"); end
      default: begin : g_def initial $display("S26 def"); end
    endcase
  endgenerate
endmodule
