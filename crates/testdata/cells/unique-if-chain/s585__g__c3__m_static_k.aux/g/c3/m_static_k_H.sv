module top;
  function int f(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
    return r;
  endfunction
  localparam int P = f(0);
  initial begin $display("P=%0d", P); #1 $finish; end
endmodule
