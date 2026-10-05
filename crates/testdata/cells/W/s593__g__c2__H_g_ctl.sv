`timescale 1ns/1ns
module sub #(parameter type T = logic signed [7:0], parameter T X = -8'sd4);
  if (X < 0) begin : gt initial $display("gif=neg"); end else begin : ge initial $display("gif=nonneg"); end
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
