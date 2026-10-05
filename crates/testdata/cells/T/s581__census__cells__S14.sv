module top;
  generate
    case (64'hFFFF_FFFF_FFFF_FFFF)
      -64'sd1: begin : g_a initial $display("S14 a"); end
      default: begin : g_def initial $display("S14 def"); end
    endcase
  endgenerate
endmodule
