module top;
  logic a;
  wire w0, w1, w2, w3;
  function automatic logic g(input logic x); integer r; r = $random; $display("g t=%0t x=%b", $time, x); return ~x; endfunction
  assign w3 = g(w2);
  assign w2 = g(w1);
  assign w1 = g(w0);
  assign w0 = g(a);
  initial a = 1'b0;
  initial #1 $display("t1 w3=%b", w3);
  initial #1 $finish;
endmodule
