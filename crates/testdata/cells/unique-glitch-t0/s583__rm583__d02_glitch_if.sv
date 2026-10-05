module top;
  logic a = 1;
  logic [1:0] y;
  always_comb begin
    y = 0;
    unique if (a) y = 1;
  end
  initial begin
    #5 a = 0;
    #0 a = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
