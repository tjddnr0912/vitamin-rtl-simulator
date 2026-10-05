`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0], parameter T X = '0);
  if (X < 0) begin : gt initial $display("gif=neg"); end else begin : ge initial $display("gif=nonneg"); end
endmodule
module top;
  sub #(.T(logic signed [7:0]), .X(-8'sd4)) u();
  initial #100 $finish;
endmodule
