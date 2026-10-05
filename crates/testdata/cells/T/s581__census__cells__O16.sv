module sub #(parameter P = 4'd15);
  generate
    case (P)
      -1: begin : g_a initial $display("O16 a"); end
      default: begin : g_def initial $display("O16 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(4'sb1111)) u();
endmodule
