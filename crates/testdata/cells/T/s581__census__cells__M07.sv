module top;
  generate
    case (1)
      1'bx, 1: begin : g_a initial $display("M07 a"); end
      default: begin : g_def initial $display("M07 def"); end
    endcase
  endgenerate
endmodule
