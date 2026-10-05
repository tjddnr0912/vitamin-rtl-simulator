module top;
  logic a, b; logic [1:0] y;
  initial begin
    a = 0; b = 0; y = 0;
    #1 unique if (a) y = 1; else if (b) y = 2;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 unique if (a) y = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
