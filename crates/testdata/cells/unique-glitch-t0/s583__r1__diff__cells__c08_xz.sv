module top;
  logic a, b; logic [1:0] y;
  initial begin
    y = 0;
    #1 a = 1'bx; b = 1'b0; unique if (a) y = 1; else if (b) y = 2;
    $display("t=%0t x-0 y=%0d", $time, y);
    #1 a = 1'bz; b = 1'bx; priority if (a) y = 1; else if (b) y = 2;
    $display("t=%0t z-x y=%0d", $time, y);
    #1 a = 1'bx; b = 1'b1; unique if (a) y = 1; else if (b) y = 2;
    $display("t=%0t x-1 y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
