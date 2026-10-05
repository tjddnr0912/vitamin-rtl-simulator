module top;
  logic a;
  wire w;
  logic v;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign w = f(a);
  always_comb v = (w === 1'bz);
  always @(posedge v) $display("PV t=%0t v=%b", $time, v);
  always @(negedge v) $display("NV t=%0t v=%b", $time, v);
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial a = 1'b0;
  initial #10 $finish;
endmodule
