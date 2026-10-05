`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  function automatic int g(input int i); return (AS[0] + 8'd0); endfunction
  localparam L = g(0);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
