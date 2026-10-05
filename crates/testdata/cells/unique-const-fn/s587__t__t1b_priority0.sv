module top;
  function automatic int p0(input int a);
    p0 = 7;
    priority0 if (a == 1) p0 = 10;
  endfunction
  localparam int K = p0(2);
  initial begin #1 $display("K=%0d", K); $finish; end
endmodule
