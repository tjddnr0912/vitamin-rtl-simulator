module top;
  logic [1:0] r;
  logic [1:0] y;
  initial begin
    $display("init t=%0t", $time);
    r = 2'b01;
    #2 r = 2'b00;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  always_comb begin
    $display("eval t=%0t r=%b", $time, r);
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
  initial #100 $finish;
endmodule
