module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int A [0:f(2)] = '{10,11,12,13,14,15,16,17};
  localparam int Q = A[7];
  initial begin #1 $display("n=%0d a7=%0d q=%0d", $size(A), A[7], Q); $finish; end
endmodule
