module top;
  generate
    case (33'h1FFFFFFFF)
      -1: begin : g_a initial $display("S17 a"); end
      default: begin : g_def initial $display("S17 def"); end
    endcase
  endgenerate
endmodule
