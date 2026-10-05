module sub #(parameter P = 4'sb1111);
  generate
    case (P)
      -1: begin : g_a initial $display("O15 a"); end
      default: begin : g_def initial $display("O15 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(4'd15)) u();
endmodule
