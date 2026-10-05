module top;
  generate
    case (0)
      (4'bx100 ==? 4'b1?00) >> 1: begin : g_a initial $display("T3 a"); end
      default: begin : g_def initial $display("T3 def"); end
    endcase
  endgenerate
endmodule
