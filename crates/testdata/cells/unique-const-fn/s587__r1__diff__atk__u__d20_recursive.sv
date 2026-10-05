module top;
  function automatic int r(input int n);
    r = 0;
    unique if (n == 100) r = 1;
    else if (n == 101) r = 2;
    if (n > 0) r = r + r(n - 1) + 1;
  endfunction
  localparam int P = r(5);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
