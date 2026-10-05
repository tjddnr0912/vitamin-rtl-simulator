module top;
  generate
    case (0)
      (4'b1100 ==? 4'b0?00): begin : g_a initial $display("X12 a"); end
      default: begin : g_def initial $display("X12 def"); end
    endcase
  endgenerate
endmodule
