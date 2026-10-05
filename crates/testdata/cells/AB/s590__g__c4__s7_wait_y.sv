module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    wait (y == 2'b01) $display("W t=%0t y=%b", $time, y);
  end
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
