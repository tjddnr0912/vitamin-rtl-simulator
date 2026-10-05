`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  if ((fl(0) + ((X + {N{1'b0}}) == 8'hFC)) > 0) begin : g initial #1 $display("GI=then"); end
  else begin : g initial #1 $display("GI=else"); end
  initial #50 $finish;
endmodule
