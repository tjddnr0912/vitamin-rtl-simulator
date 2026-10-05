module top;
  logic [1:0] y;
  always_latch if (1'b1) y = 2'd2;
  initial begin
    $display("i0 y=%b", y);
    #0 $display("i1 y=%b", y);
    #0 $display("i2 y=%b", y);
    #0 $display("i3 y=%b", y);
  end
  initial #1 $finish;
endmodule
