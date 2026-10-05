module top;
  logic a;
  wire w;
  wire [1:0] u;
  wire v;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign w = f(a);
  assign u = {w, 1'b0};
  assign v = (u[1] === 1'bz);
  always @(posedge v) $display("PV t=%0t v=%b", $time, v);
  always @(negedge v) $display("NV t=%0t v=%b", $time, v);
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial a = 1'b0;
  initial #10 $finish;
endmodule
