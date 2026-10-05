module top;
  function automatic int f(input int a);
    f = 7;
    priority if (a == 1) f = 10;
  endfunction
  localparam int A [2] = '{f(2), 1};
  initial begin #1 $display("A0=%0d", A[0]); $finish; end
endmodule
