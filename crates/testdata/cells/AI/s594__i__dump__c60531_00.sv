`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  if ((X + {N{1'b0}}) == (fl(0) + 8'd252)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #50 $finish;
endmodule
