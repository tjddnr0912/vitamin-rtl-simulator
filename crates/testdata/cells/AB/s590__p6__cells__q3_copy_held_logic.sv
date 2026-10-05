module top;
  logic [1:0] a;
  wire [1:0] w;
  logic [1:0] c;
  function logic [1:0] f(input logic [1:0] x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign c = w;
  always @(c) $display("C t=%0t c=%b", $time, c);
  initial begin
    a = 2'b01;
    $display("i0 w=%b c=%b", w, c);
    #1 $display("i1 w=%b c=%b", w, c);
    $finish;
  end
  initial #100 $finish;
endmodule
