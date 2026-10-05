module top;
  logic a;
  wire w0, w1, w2, w3, w4, w5;
  function automatic logic g(input logic x); integer r; r = $random; $display("g t=%0t x=%b r=%0d", $time, x, r); return ~x; endfunction
  assign w0 = g(a);
  assign w1 = g(w0);
  assign w2 = g(w1);
  assign w3 = g(w2);
  assign w4 = g(w3);
  assign w5 = g(w4);
  initial a = 1'b0;
  initial #1 $display("t1 w5=%b rnd=%0d", w5, $random);
  initial #2 $finish;
endmodule
