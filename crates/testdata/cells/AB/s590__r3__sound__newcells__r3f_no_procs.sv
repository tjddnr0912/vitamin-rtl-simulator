module top;
  logic a = 1'b0;
  wire y, z, d, w;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign y = f(a);
  assign z = f(y);
  assign #1 d = z;
  assign w = f(d);
endmodule
