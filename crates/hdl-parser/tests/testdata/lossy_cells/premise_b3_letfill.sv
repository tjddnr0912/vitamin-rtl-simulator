module t; logic [69:0] w1, w2, w3, w4; logic c;
  let LX = 'x; let ID(q) = q; let A(q) = q | 70'h0; let T(q) = c ? q : 70'h0;
  initial begin c = 1; w1 = LX; w2 = ID('z); w3 = A('z); w4 = T('x); #1 $display("A w1=%h w2=%h w3=%h w4=%h", w1, w2, w3, w4); end
  initial #100 $finish;
endmodule
