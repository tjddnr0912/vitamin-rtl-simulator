module top;
  logic a;
  wire y, z;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); if (x === 1'b0) $fatal(1, "F0"); return x; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return x; endfunction
  assign z = g(y);
  assign y = f(a);
  initial a = 1'b0;
  final $display("final y=%b z=%b", y, z);
  initial #10 $finish;
endmodule
