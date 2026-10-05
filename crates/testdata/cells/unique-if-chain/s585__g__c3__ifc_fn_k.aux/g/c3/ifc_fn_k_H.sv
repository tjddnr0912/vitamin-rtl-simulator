interface I;
  function int f(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
    return r;
  endfunction
  localparam int P = f(0);
  initial begin $display("P=%0d", P); end
endinterface
module top;
  I i();
  initial #1 $finish;
endmodule
