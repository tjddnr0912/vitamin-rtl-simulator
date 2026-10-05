`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  logic [fl(0) + ((X + {N{1'b0}}) == 8'hFC) + 3 : 0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #50 $finish;
endmodule
