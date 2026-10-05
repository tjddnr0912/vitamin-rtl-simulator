module top;
  logic a, b;
  function automatic logic f(input logic x);
    if (x === 1'bz) $display("never");
    f = x;
  endfunction
  wire y = f(a);
  wire y2 = f(b);
  initial begin $dumpfile("iv.vcd"); $dumpvars(0, top); end
  initial begin b = 1; #2 a = 1; end
  always @(y) $display("y=%b t=%0t", y, $time);
  initial #1 $display("#1 y=%b y2=%b", y, y2);
  initial #4 $finish;
endmodule
