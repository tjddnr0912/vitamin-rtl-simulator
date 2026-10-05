module top;
  generate
    case (0)
      (4'bx100 ==? 4'b1?00), 0: begin : g_a initial $display("M10 a"); end
      default: begin : g_def initial $display("M10 def"); end
    endcase
  endgenerate
endmodule
