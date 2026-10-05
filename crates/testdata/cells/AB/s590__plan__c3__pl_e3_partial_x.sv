module top;
  logic a; wire [1:0] w; logic [1:0] y;
  function logic [1:0] f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return {x, 1'b1};
  endfunction
  assign w = f(a);
  assign y = f(a);
  always @(posedge w[0]) $display("P t=%0t w=%b", $time, w);
  always @(w) $display("W t=%0t w=%b", $time, w);
  always @(w[1]) $display("W1 t=%0t w=%b", $time, w);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(posedge y[0]) $display("PY t=%0t y=%b", $time, y);
  initial begin
    $display("i0 w=%b y=%b", w, y);
    #1 $display("t=%0t w=%b y=%b", $time, w, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
