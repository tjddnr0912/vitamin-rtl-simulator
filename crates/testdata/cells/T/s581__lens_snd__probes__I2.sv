module sub #(parameter P = 4'b1111) ();
  localparam L = ((P + 4'd1) ==? 4'b000?);
  initial $display("I2 L=%0d", L);
endmodule
module top;
  sub #(.P(16'h00FF)) u();
endmodule
