module top;
  logic a;
  wire [7:0] y;
  function [7:0] cnt_f(input logic x);
    integer c = 0;
    c = c + 1;
    cnt_f = c[7:0];
  endfunction
  assign y = cnt_f(a);
  initial a = 1'b0;
  initial #1 $display("t1 y=%0d", y);
  initial #10 $finish;
endmodule
