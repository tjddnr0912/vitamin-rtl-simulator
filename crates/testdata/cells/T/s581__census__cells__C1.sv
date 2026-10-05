module top;
  generate
    case (1)
      (4'bx100 ==? 4'b1?00): begin : g_a initial $display("C1 a"); end
      default: begin : g_def initial $display("C1 def"); end
    endcase
  endgenerate
endmodule
