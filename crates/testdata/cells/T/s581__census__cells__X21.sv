module top;
  generate
    case (1)
      (4'bx100 !=? 4'b1?00): begin : g_a initial $display("X21 a"); end
      default: begin : g_def initial $display("X21 def"); end
    endcase
  endgenerate
endmodule
