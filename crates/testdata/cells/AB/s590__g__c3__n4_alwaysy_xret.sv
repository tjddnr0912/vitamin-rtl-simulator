module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  assign w = f(a, b);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(w) $display("W t=%0t w=%b", $time, w);
  always @(posedge w[0]) $display("P t=%0t w=%b", $time, w);
  always @(negedge w[0]) $display("N t=%0t w=%b", $time, w);
  initial begin
    $display("i0 y=%b w=%b", y, w);
    #1 $display("t=%0t y=%b w=%b", $time, y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
