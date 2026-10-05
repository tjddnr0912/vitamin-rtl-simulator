module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    if (id == 3 && v === 1'b0) $finish;
    f = ~v;
  endfunction
  wire t1, c0, c1, o1;
  assign o1 = f(c1, 4);
  assign c1 = f(c0, 3);
  assign c0 = f(t1 | c1, 2);
  assign t1 = f(a, 1);
  always @(o1) $display("ev o1=%b t=%0t", o1, $time);
  initial begin a = 0; #1 $display("unreached o1=%b", o1); end
  initial #10 $finish;
endmodule
