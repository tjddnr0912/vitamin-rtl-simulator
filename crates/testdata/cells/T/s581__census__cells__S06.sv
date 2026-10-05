module top;
  generate
    case (-1)
      32'hFFFFFFFF: begin : g_a initial $display("S06 a"); end
      default: begin : g_def initial $display("S06 def"); end
    endcase
  endgenerate
endmodule
