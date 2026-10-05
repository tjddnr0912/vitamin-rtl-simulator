module top;
  logic a, b;
  wire d, m, w;
  function automatic logic p(input logic x); return ~x; endfunction
  assign w = p(a);
  assign #1 d = w;
  assign m = a;
  assign m = b;
  always @(d or m) $display("E t=%0t d=%b m=%b", $time, d, m);
  initial begin $dumpfile("d.vcd"); $dumpvars(0, top); a = 1'b0; b = 1'bz; #3 a = 1'b1; #3 b = 1'b0; #3 $finish; end
endmodule
