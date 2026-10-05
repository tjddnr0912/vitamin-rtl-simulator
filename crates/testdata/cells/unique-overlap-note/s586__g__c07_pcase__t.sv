module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b11;
    #1;
    priority casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
