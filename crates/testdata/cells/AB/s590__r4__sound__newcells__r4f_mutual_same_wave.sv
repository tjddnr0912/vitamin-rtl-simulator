module top;
  logic a, b;
  wire v1, v2;
  function automatic logic h1(input logic x); integer r; r = $random; $display("h1 t=%0t v2=%b", $time, v2); return x | v2; endfunction
  function automatic logic h2(input logic x); integer r; r = $random; $display("h2 t=%0t v1=%b", $time, v1); return x & v1; endfunction
  assign v2 = h2(b);
  assign v1 = h1(a);
  initial begin a = 1'b1; b = 1'b1; end
  final $display("final v1=%b v2=%b", v1, v2);
  initial #3 $finish;
endmodule
