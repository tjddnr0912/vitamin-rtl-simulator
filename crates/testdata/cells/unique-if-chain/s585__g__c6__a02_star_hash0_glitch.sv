module top;
  logic [1:0] r = 2'b01;
  logic [1:0] y;
  always @* begin
    $display("eval t=%0t r=%b", $time, r);
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
  initial begin
    #5 r = 2'b00;
    #0 r = 2'b01;
    #1 $display("t=%0t y=%0d done", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
