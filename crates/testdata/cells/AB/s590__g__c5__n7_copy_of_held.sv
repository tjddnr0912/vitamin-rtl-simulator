module top;
  logic a, b; wire [1:0] w, w2;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign w = f(a, b);
  assign w2 = w;
  always @(w2) $display("W2 t=%0t w2=%b", $time, w2);
  always @(w) $display("W t=%0t w=%b", $time, w);
  initial begin
    $display("i0 w=%b w2=%b", w, w2);
    #1 $display("t=%0t w=%b w2=%b", $time, w, w2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
