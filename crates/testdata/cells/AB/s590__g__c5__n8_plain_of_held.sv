module top;
  logic a, b; wire [1:0] w, v;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign w = f(a, b);
  assign v = ~w;
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial begin
    $display("i0 w=%b v=%b", w, v);
    #1 $display("t=%0t w=%b v=%b", $time, w, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
