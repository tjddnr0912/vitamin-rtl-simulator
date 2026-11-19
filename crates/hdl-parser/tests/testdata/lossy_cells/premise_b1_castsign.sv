module t; logic [69:0] v1, v2, v3, v4, v5;
  initial begin
    v1 = $signed('x); v2 = $signed('1); v3 = 70'('z); v4 = 'x + 70'd1; v5 = $unsigned('z);
    #1 $display("A v1=%h v2=%h v3=%h v4=%h v5=%h", v1, v2, v3, v4, v5);
  end
  initial #100 $finish;
endmodule
