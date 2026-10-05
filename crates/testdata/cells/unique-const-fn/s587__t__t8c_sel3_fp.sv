module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [15:0][3:0] m = 64'h0123456789ABCDEF;
  initial begin #1 $display("m=%h", m[f(2) +: 2]); $finish; end
endmodule
