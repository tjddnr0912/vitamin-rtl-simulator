module sub #(parameter P = 0);
  generate
    case (P)
      4'b1111: begin : g_a initial $display("O03 a"); end
      default: begin : g_def initial $display("O03 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(4'sb1111)) u();
endmodule
