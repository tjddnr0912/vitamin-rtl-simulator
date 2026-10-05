module top;
  function automatic int cf(input int x, input int z);
    cf = 0;
    if (x == 1) cf = 1; else unique if (z == 1) cf = 2;
  endfunction
  localparam int P1 = cf(0, 1);
  logic [1:0] y;
  initial begin y = cf(0, 0); $display("P1=%0d y=%0d", P1, y); #1 $finish; end
endmodule
