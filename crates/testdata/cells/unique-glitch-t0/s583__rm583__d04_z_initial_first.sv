module top;
  logic [1:0] r;
  logic [1:0] y;
  initial begin
    r = 2'b01;
    #2 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  always_comb begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
  initial #100 $finish;
endmodule
