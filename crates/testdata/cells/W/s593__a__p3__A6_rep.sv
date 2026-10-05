`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction
  wire [7:0] r = {((fr(2) inside {4'b0?00})+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
