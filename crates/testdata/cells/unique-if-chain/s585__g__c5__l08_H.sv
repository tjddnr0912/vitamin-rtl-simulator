module top;
  function automatic int cf(input int x, input int z);
    cf = 0;
    if (x == 1) cf = 1; else unique if (z == 1) cf = 2;
  endfunction
  localparam int P0 = cf(0, 0);
  localparam int P1 = cf(1, 1);
  initial begin $display("P0=%0d P1=%0d", P0, P1); #1 $finish; end
endmodule
