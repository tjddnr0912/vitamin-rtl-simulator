module top;
  logic a;
  wire g1;
  function automatic logic f2(input logic v);
    $display("f2 t=%0t v=%b", $time, v);
    f2 = ~v;
  endfunction
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b g1=%b", id, $time, v, g1);
    f = v ^ g1;
  endfunction
  wire r = f(a, 1);
  assign g1 = f2(a);
  always @(r) $display("ev r=%b t=%0t", r, $time);
  initial begin a = 1; #1 $display("t1 g1=%b r=%b", g1, r); $finish; end
  initial #10 $finish;
endmodule
