`timescale 1ns/1ns
module t;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  int k;
  initial begin k = 0; repeat (2'b00 + fl(2)) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
