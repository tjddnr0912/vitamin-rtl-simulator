module top;
  generate
    case (4'b1100)
      {2'b11, 2'bx0} | 4'b0001: begin : g_a initial $display("X19 a"); end
      4'b1100: begin : g_b initial $display("X19 b"); end
      default: begin : g_def initial $display("X19 def"); end
    endcase
  endgenerate
endmodule
