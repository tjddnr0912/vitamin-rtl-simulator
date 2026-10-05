module t;
  logic [1:0] r; int y, z;
  initial begin
    r = 2'b11;
    #1;
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    unique0 casez (r)
      2'b?1: z = 1;
      2'b1?: z = 2;
    endcase
    $display("y=%0d z=%0d", y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
