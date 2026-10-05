module top;
  function void fv(input int x, output int r);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
  function int cf(input int x); int q; fv(x, q); return q; endfunction
  localparam int P = cf(0);
  initial begin $display("P=%0d", P); #1 $finish; end
endmodule
