module top;
  logic a, b;
  wire n, q, d, m;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign n = f(a);
  assign n = f(b);
  assign q = f(n);
  assign #1 d = q;
  assign m = a;
  assign m = b;
  initial begin $dumpfile("e.vcd"); $dumpvars(0, top); a = 1'b0; b = 1'b0; #3 a = 1'b1; #3 $finish; end
endmodule
