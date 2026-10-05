module top;
  logic a = 0, b;
  logic [1:0] y;
  always_comb begin
    unique if (a && b) y = 3; else if (!a && !b) y = 0;
  end
  always_comb b = a;
  initial begin
    #5 a = 1;
    #1 $display("t=%0t y=%0d done", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
