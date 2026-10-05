module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = ~v;
  endfunction
  wire p1, p2, p3, c0, c1, j;
  assign j  = f(p3 & c1, 9);
  assign c1 = f(c0, 5);
  assign c0 = f(p1 | c1, 4);
  assign p3 = f(p2, 3);
  assign p2 = f(p1, 2);
  assign p1 = f(a, 1);
  initial begin a = 0; #1 $display("t1 p1=%b p2=%b p3=%b c0=%b c1=%b j=%b", p1, p2, p3, c0, c1, j); $finish; end
  initial #10 $finish;
endmodule
