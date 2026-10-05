module top;
  generate
    case (65'h1_FFFF_FFFF_FFFF_FFFF)
      '1: begin : g_a initial $display("F04 a"); end
      default: begin : g_def initial $display("F04 def"); end
    endcase
  endgenerate
endmodule
