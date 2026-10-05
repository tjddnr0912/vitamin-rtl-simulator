module top;
  wire w = f(1'b1);
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  always @(posedge w) $display("P t=%0t w=%b", $time, w);
  always @(w) $display("W t=%0t w=%b", $time, w);
  initial begin
    $display("i0 w=%b", w);
    @(w) $display("B t=%0t w=%b", $time, w);
  end
  initial begin
    #1 $display("t=%0t w=%b", $time, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
