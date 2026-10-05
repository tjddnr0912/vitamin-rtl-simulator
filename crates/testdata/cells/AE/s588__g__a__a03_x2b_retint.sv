module top;
  function automatic int fi(input int a);
    if (a == 1) fi = 10;
  endfunction
  localparam int P = fi(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
