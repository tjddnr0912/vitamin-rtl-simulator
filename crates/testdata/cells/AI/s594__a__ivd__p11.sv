`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic logic [7:0] fd(input int a); fd = a; endfunction
  function automatic logic [7:0] fl(input int a); logic [7:0] q; fl = q + a; endfunction
  reg r = 0;
  initial #10 r = 1;
  wire w;
  assign #(fd(2)) w = r;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #600 $finish;
endmodule
