module top;
  logic a = 0, b = 0;
  function void g();
    if (a) ; else unique if (b) ;
  endfunction
  function int f(input int k);
    g();
    return k + 1;
  endfunction
  localparam int P = f(6);
  initial begin #1 $display("P=%0d", P); #1 $finish; end
endmodule
