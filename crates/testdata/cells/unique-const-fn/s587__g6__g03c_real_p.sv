module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam real R = f(2);
  initial begin #1 $display("R=%0.2f", R); $finish; end
endmodule
