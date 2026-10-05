`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  function automatic int f2(input int a); return a; endfunction
  wire w;
  assign #({f2(2){1'b0}} + 8'd5) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #300 $finish;
endmodule
