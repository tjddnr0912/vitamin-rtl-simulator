module sub #(parameter P = 4'b1111) ();
  case (1)
    ((P + 4'd1) ==? 4'b000?): begin : g_a initial $display("I4 a"); end
    default: begin : g_d initial $display("I4 def"); end
  endcase
endmodule
module top;
  sub #(.P(16'h00FF)) u();
endmodule
