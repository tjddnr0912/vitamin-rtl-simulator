module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    if (id == 2 && v === 1'b1) $fatal(1, "boom id=%0d v=%b", id, v);
    f = v;
  endfunction
  wire m, n, k;
  assign k = f(n, 3);
  assign n = f(m, 2);
  assign m = f(a, 1);
  always @(k) $display("ev k=%b t=%0t", k, $time);
  initial begin a = 1; #1 $display("unreached t1 k=%b", k); $finish; end
  initial #10 $finish;
endmodule
