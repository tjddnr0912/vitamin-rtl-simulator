`timescale 1ns/1ns
module t;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  initial #1 $display("rep=%h", {(2'b00 + fl(2)){4'hF}});
  initial #40 $finish;
endmodule
