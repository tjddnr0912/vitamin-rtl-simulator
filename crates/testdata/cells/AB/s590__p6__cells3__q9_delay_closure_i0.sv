module top;
  logic a; wire w; wire v;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign #1 v = w;
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial begin
    $display("i0 v=%b", v);
    a = 1;
    #0 $display("i1 v=%b", v);
    #2 $display("t=%0t v=%b", $time, v);
    $finish;
  end
  initial #100 $finish;
endmodule
