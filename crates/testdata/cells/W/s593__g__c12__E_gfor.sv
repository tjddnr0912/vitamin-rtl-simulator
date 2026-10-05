`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  for (genvar i = 0; i < (X ==? 4'b1?00) + 2; i++) begin : g
    initial $display("gi=%0d", i);
  end
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
