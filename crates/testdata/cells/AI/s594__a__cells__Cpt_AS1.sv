`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  initial begin #(AS[1] * 1ns); $display("fired t=%0t", $time); end
  initial #400 $finish;
endmodule
