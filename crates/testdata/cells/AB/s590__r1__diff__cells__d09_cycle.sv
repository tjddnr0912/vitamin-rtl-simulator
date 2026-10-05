module top;
  logic a;
  function automatic logic fa(input logic x, input logic q);
    $display("fa x=%b q=%b t=%0t", x, q, $time);
    fa = x | (q & 1'b0);
  endfunction
  function automatic logic fb(input logic p);
    $display("fb p=%b t=%0t", p, $time);
    fb = p;
  endfunction
  wire p, q;
  assign p = fa(a, q);
  assign q = fb(p);
  initial a = 1;
  initial #1 $display("p=%b q=%b", p, q);
  initial #3 $finish;
endmodule
