module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  let L(a) = f(a) + 1;
  localparam int P = L(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
