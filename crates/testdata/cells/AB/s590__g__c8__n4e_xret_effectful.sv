module top;
  logic a, b; logic [1:0] y; wire [1:0] w, w2, v;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  assign w = f(a, b);
  assign w2 = w;
  assign v = ~w;
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(w) $display("W t=%0t w=%b", $time, w);
  always @(w2) $display("W2 t=%0t w2=%b", $time, w2);
  always @(v) $display("V t=%0t v=%b", $time, v);
  always @(posedge w[0]) $display("P t=%0t w=%b", $time, w);
  initial begin
    $display("i0 y=%b w=%b w2=%b v=%b", y, w, w2, v);
    #1 $display("t=%0t y=%b w=%b w2=%b v=%b", $time, y, w, w2, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
