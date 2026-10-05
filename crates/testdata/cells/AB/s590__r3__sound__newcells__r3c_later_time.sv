module top;
  logic a, c;
  wire y, z, d, m;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return x; endfunction
  assign y = f(a);
  assign z = g(y);
  assign #2 d = z;
  assign m = z;
  assign m = c;
  always @(z or d or m) $display("E t=%0t z=%b d=%b m=%b", $time, z, d, m);
  initial begin a = 1'b0; c = 1'bz; #5 a = 1'b1; #5 a = 1'bx; #5 c = 1'b0; end
  initial #30 $finish;
endmodule
