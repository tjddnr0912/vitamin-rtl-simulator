module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = ~v;
  endfunction
  function automatic logic gr(input logic v);
    integer r; r = $random;
    $display("gr t=%0t v=%b", $time, v);
    gr = ~v;
  endfunction
  wire t1, c0, c1, o1;
  assign o1 = f(c1, 4);
  assign c1 = gr(c0);
  assign c0 = f(t1 | c1, 2);
  assign t1 = f(a, 1);
  initial begin a = 0; #1 $display("t1 c0=%b c1=%b o1=%b", c0, c1, o1); $finish; end
  initial #10 $finish;
endmodule
