module top;
  logic a, s; wire [1:0] w;
  function logic [1:0] f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return {x, ~x};
  endfunction
  assign w = f(a);
  always @(s) $display("S t=%0t s=%b w=%b", $time, s, w);
  always @(w) $display("W t=%0t w=%b", $time, w);
  initial begin
    a = 1; s = 1;
    #0 $display("Z t=%0t w=%b", $time, w);
    #1 $display("t=%0t w=%b", $time, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
