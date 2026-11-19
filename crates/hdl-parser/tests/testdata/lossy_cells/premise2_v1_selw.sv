module t; logic [3:0][69:0] pk, pk2, pk3, pk4; logic [69:0] m2d [2][3]; int k, i, j;
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t; s_t [2:0] ps;
  initial begin k = 1; i = 1; j = 2; pk = '0; pk2 = '0; pk3 = '0; pk4 = '0; ps = '0;
    pk[k] = 'x; m2d[i][j] = 'z;
    #1 $display("A pk=%h", pk); $display("B pk2=%h", pk2); $display("C pk3=%h", pk3); $display("D pk4=%h", pk4);
    $display("E m=%h ps=%h", m2d[1][2], ps); end
  initial #100 $finish;
endmodule
