`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  localparam L = ({fx(2), 4'b0000} inside {8'b0?00_0000});
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
