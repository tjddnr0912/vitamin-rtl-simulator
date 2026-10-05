`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  wire w;
  assign #({N{1'b0}} + fl(2)) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #40 $finish;
endmodule
