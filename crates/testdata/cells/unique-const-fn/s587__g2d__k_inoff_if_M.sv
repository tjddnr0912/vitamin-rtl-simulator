module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [3:0][15:0][3:0] m = '0;
  initial begin m[1] = 64'h0123456789ABCDEF; #1 $display("m=%h", m[1][f(2) - 7 +: 2]); $finish; end
endmodule
