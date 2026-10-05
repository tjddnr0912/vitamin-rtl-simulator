function int f(input int x);
  int r; r = 7;
  unique if (x == 1) r = 1; else if (x == 2) r = 2;
  return r;
endfunction
module top;
  localparam int P = f(0);
  initial begin $display("P=%0d", P); #1 $finish; end
endmodule
