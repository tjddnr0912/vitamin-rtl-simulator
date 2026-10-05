module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] x = 32'hFFFF_FFFF;
  initial begin #1 $display("cw=%0d", $bits(f(2)'(x[3:0]))); $finish; end
endmodule
