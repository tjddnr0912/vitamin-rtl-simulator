module sub(input logic p [f(2):0]);
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial #1 $display("l=%0d r=%0d i=%0d p7=%b p0=%b", $left(p), $right(p), $increment(p), p[7], p[0]);
endmodule
module top;
  logic arr [7:0];
  initial begin foreach (arr[i]) arr[i] = 1'b0; arr[7] = 1'b1; end
  sub u(.p(arr));
  initial #2 $finish;
endmodule
