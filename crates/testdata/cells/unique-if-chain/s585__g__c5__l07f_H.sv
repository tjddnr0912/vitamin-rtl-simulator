module top;
  logic a, b; logic [1:0] y1, y2, y3;
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    if (x) f = 1; else unique if (z) f = 2;
  endfunction
  assign y3 = f(a, b);
  always_comb y2 = f(a, b);
  initial begin
    a = 1; b = 0;
    #1 a = 0;
    #1 y1 = f(a, b); $display("t=%0t y1=%0d y2=%0d y3=%0d", $time, y1, y2, y3);
    #1 $finish;
  end
endmodule
