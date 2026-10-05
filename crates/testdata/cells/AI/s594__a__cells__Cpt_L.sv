`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  initial begin #((X + 2'b00) * 1ns); $display("fired t=%0t", $time); end
  initial #400 $finish;
endmodule
