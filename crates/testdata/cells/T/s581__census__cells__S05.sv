module top;
  generate
    case (-1)
      -1: begin : g_a initial $display("S05 a"); end
      default: begin : g_def initial $display("S05 def"); end
    endcase
  endgenerate
endmodule
