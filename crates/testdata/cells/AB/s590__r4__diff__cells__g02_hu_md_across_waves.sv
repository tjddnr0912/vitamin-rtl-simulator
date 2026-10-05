module top;
  logic a, e1, e2;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = ~v;
  endfunction
  function automatic logic gr(input logic v);
    integer r; r = $random;
    $display("gr t=%0t v=%b", $time, v);
    gr = v;
  endfunction
  wire c1 = f(a, 1);
  wire c2 = f(c1, 2);
  tri w;
  assign w = e1 ? gr(a) : 1'bz;
  assign w = e2 ? c2 : 1'bz;
  wire y = f(w, 9);
  always @(w) $display("ev w=%b t=%0t", w, $time);
  initial begin a = 1; e1 = 1; e2 = 0; #1 $display("t1 w=%b y=%b", w, y); #1 e1 = 0; e2 = 1; #1 $display("t3 w=%b y=%b", w, y); $finish; end
  initial #10 $finish;
endmodule
