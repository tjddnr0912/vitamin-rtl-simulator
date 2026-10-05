module top;
  logic [1:0] r;
  logic en;
  logic [1:0] y;
  always_latch begin
    if (en)
      unique casez (r)
        2'b?1: y = 1;
        2'b1?: y = 2;
      endcase
  end
  initial begin
    en = 1; r = 2'b01;
    #2 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
