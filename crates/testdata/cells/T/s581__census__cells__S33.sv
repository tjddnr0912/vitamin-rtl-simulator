module top;
  generate
    case (64'hFFFF_FFFF_FFFF_FFFF)
      64'hFFFF_FFFF_FFFF_FFFF: begin : g_a initial $display("S33 a"); end
      default: begin : g_def initial $display("S33 def"); end
    endcase
  endgenerate
endmodule
