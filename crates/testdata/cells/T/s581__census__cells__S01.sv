module top;
  generate
    case (1'bx)
      1'bx: begin : g_a initial $display("S01 a"); end
      default: begin : g_def initial $display("S01 def"); end
    endcase
  endgenerate
endmodule
