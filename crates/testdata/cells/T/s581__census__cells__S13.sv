module top;
  generate
    case (64'hFFFF_FFFF_FFFF_FFFF)
      -1: begin : g_a initial $display("S13 a"); end
      default: begin : g_def initial $display("S13 def"); end
    endcase
  endgenerate
endmodule
