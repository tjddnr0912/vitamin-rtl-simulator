module top;
  logic en1, en2, a, b, en3, c;
  function automatic logic f(input logic x);
    $display("f x=%b t=%0t", x, $time);
    f = x;
  endfunction
  tri w;
  assign w = en1 ? f(a) : 1'bz;
  assign w = en2 ? b : 1'bz;
  tri1 t1;
  assign t1 = en3 ? f(c) : 1'bz;
  wand wa;
  assign wa = f(a);
  assign wa = b;
  always @(w) $display("w=%b t=%0t", w, $time);
  always @(t1) $display("t1=%b t=%0t", t1, $time);
  always @(wa) $display("wa=%b t=%0t", wa, $time);
  initial begin en1 = 1; a = 1; en2 = 0; b = 1; en3 = 0; c = 0; end
  initial #1 $display("#1 w=%b t1=%b wa=%b", w, t1, wa);
  initial begin #2 en1 = 0; en2 = 1; b = 0; end
  initial #4 $finish;
endmodule
