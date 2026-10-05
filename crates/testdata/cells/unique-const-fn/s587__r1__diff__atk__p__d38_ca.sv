module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  int w;
  assign w = f(2);
  localparam int P = f(2);
  initial begin #1 $display("w=%0d P=%0d", w, P); $finish; end
endmodule
