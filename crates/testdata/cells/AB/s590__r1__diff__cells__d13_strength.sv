module top;
  logic a, sb;
  function automatic logic f(input logic x);
    $display("f x=%b t=%0t", x, $time);
    f = x;
  endfunction
  wire s;
  assign (weak0, weak1) s = f(a);
  assign (strong0, strong1) s = sb;
  always @(s) $display("s=%b t=%0t", s, $time);
  initial begin a = 1; sb = 0; end
  initial #2 sb = 1'bz;
  initial #1 $display("#1 s=%b", s);
  initial #3 $display("#3 s=%b", s);
  initial #4 $finish;
endmodule
