module t; logic [69:0] a1,a2,a3,a4,a5,a6,a7,a8,a9; logic c,d;
  initial begin c=1; d=1;
    a1 = ~'0; a2 = ('1); a3 = ('z); a4 = c ? (d ? 'z : 'x) : 70'h0; a5 = c ? '1 : '0; a6 = ~'z; a7 = c ? 'z : 'x; a8 = (c ? ('1) : ('0)); a9 = -'1;
    #1 $display("A a1=%h a2=%h a3=%h a4=%h a5=%h", a1,a2,a3,a4,a5); $display("B a6=%h a7=%h a8=%h a9=%h", a6,a7,a8,a9); end
  initial #100 $finish;
endmodule
