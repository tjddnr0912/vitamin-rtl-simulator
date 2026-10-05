module sub #(parameter P = 4'd0);
  generate
    case (P)
      -1: begin : g_a initial $display("O17 a"); end
      default: begin : g_def initial $display("O17 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub u();
  defparam u.P = 4'sb1111;
endmodule
