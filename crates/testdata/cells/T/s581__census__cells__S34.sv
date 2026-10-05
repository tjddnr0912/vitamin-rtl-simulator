module top;
  generate
    case (-64'sd1)
      65'h1FFFFFFFFFFFFFFFF: begin : g_a initial $display("S34 a"); end
      default: begin : g_def initial $display("S34 def"); end
    endcase
  endgenerate
endmodule
