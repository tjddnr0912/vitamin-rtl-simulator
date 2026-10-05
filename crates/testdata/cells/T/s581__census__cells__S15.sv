module top;
  generate
    case (-1)
      64'hFFFF_FFFF_FFFF_FFFF: begin : g_a initial $display("S15 a"); end
      default: begin : g_def initial $display("S15 def"); end
    endcase
  endgenerate
endmodule
