`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  wire [7:0] r = {(($clog2(fx(2) + 4'd1) inside {32'b0000_0000_0000_0000_0000_0000_0000_000?})+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
