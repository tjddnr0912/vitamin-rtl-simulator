module top;
  logic a, b; logic [1:0] y;
  always @* begin
    unique if (a) y = 1;
    else if (b) y = 2;
  end
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $finish;
  end
endmodule
