module top;
  logic a; wire w; wire v; logic [1:0] n = 0;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign v = (w === 1'bz);
  always @(v) begin n = n + 1; $display("V t=%0t v=%b n=%0d", $time, v, n); end
  initial begin
    a = 1;
    #1 $display("t=%0t w=%b v=%b n=%0d", $time, w, v, n);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
