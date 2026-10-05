module sub #(parameter P = 4'sb1111);
  generate
    case (4'b1111)
      P: begin : g_a initial $display("O20 a"); end
      default: begin : g_def initial $display("O20 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(4'd15)) u();
endmodule
