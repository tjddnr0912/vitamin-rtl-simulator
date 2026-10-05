module top;
  logic en1, en2, a, b;
  function automatic logic f(input logic x);
    $display("f x=%b t=%0t", x, $time);
    f = x;
  endfunction
  tri w;
  assign w = en1 ? f(a) : 1'bz;
  assign w = en2 ? b : 1'bz;
  wire v = ~w;
  always @(w) $display("w=%b t=%0t", w, $time);
  always @(v) $display("v=%b t=%0t", v, $time);
  initial begin en1 = 1; a = 1; en2 = 0; b = 1; end
  initial #1 $display("#1 w=%b v=%b", w, v);
  initial begin #2 en1 = 0; en2 = 1; b = 0; end
  initial #4 $finish;
endmodule
