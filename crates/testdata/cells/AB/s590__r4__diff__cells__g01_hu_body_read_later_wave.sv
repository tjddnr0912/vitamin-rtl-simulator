module top;
  logic a, b, z;
  wire g1;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = ~v;
  endfunction
  function automatic logic gr(input logic v);
    integer r; r = $random;
    $display("gr t=%0t v=%b g1=%b", $time, v, g1);
    gr = v ^ g1;
  endfunction
  function automatic logic gd(input logic v);
    integer r; r = $random;
    $display("gd t=%0t v=%b", $time, v);
    gd = ~v;
  endfunction
  wire c1 = f(a, 1);
  wire c2 = f(c1, 2);
  assign g1 = f(c2, 3);
  wire u = gr(b);
  wire u2 = gd(g1);
  always @(u) $display("ev u=%b t=%0t", u, $time);
  initial begin a = 0; b = 1; #1 $display("t1 g1=%b u=%b u2=%b", g1, u, u2); #2 z = 1; #1 $display("t4 u=%b", u); $finish; end
  initial #10 $finish;
endmodule
