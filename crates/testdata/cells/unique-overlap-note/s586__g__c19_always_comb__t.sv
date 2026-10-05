module t;
  logic [1:0] r; int y;
  always_comb begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
      default: y = 0;
    endcase
  end
  initial begin
    r = 2'b11;
    #1 $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
