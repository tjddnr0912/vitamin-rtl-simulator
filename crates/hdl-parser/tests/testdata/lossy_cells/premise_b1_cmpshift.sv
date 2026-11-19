module t; logic [69:0] v, s1, s2, s3; logic e1, e2, e3;
  initial begin
    v = 70'h0;
    e1 = (v == 'x); e2 = (v != 'z); e3 = (v < 'x);
    s1 = 'x >> 1; s2 = '1 >> 1; s3 = 'z << 1;
    #1 $display("A e1=%b e2=%b e3=%b s1=%h s2=%h s3=%h", e1, e2, e3, s1, s2, s3);
  end
  initial #100 $finish;
endmodule
