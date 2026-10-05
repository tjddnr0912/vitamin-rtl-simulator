module unused(input logic [1:0] r, output int y);
  always_comb begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
endmodule
module t;
  int y;
  initial begin
    y = 5;
    #1 $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
