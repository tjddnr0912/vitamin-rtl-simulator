module top;
  logic a, b, c; logic [1:0] y;
  initial begin
    a = 0; b = 0; c = 1; y = 0;
    #1 unique if (a) y = 1; else if (b) assert (c); else y = 3;
    $display("t=%0t u-semi-else y=%0d", $time, y);
    #1 y = 0; if (b) assert (c); else y = 3;
    $display("t=%0t plain-semi-else y=%0d", $time, y);
    #1 y = 0; unique if (a) y = 1; else assert #0 (b) else if (c) y = 2;
    $display("t=%0t u-deferred-else y=%0d", $time, y);
    #1 y = 0; unique if (a) y = 1; else assert (b) else unique if (a) y = 2;
    $display("t=%0t u-assert-else-unique y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
