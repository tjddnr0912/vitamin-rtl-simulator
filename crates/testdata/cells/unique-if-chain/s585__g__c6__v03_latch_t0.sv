module top;
  logic [1:0] y;
  logic en = 1;
  always_latch if (en) y = 2'd2;
  initial begin
    $display("init t=%0t y=%b", $time, y);
    #0 $display("init#0 t=%0t y=%b", $time, y);
    #0 $display("init#0#0 t=%0t y=%b", $time, y);
  end
  initial #1 $finish;
endmodule
