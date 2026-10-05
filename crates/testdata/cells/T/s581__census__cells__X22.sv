module top;
  generate
    case (0)
      (4'b1100 ==? 4'b0?00) >> 1: begin : g_a initial $display("X22 a"); end
      default: begin : g_def initial $display("X22 def"); end
    endcase
  endgenerate
endmodule
