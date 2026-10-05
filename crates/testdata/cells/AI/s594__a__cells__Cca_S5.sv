`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  wire w;
  assign #((AS[0] + 8'd0) / 8'd50) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #40 $finish;
endmodule
