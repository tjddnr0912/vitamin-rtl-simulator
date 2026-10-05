module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = ~v;
  endfunction
  wire t1, c0, c1, c2, o1, o2;
  assign o2 = f(o1, 6);
  assign o1 = f(c1, 5);
  assign c1 = f(c0, 3);
  assign c2 = f(c1, 4);
  assign c0 = f(t1 | c2, 2);
  assign t1 = f(a, 1);
  initial begin a = 0; #1 $display("t1 t1=%b c0=%b c1=%b c2=%b o1=%b o2=%b", t1, c0, c1, c2, o1, o2); $finish; end
  initial #10 $finish;
endmodule
