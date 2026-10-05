module sub #(parameter P = 8'd0);
  generate
    case (P)
      -1: begin : g_a initial $display("O19 a"); end
      default: begin : g_def initial $display("O19 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(4'sb1111)) u();
endmodule
