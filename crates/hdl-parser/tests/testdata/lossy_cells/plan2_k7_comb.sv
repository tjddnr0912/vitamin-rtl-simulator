module t; logic [69:0] w1, w2, w3, w4; logic [1:0] s; typedef enum logic [69:0] {E0 = 0} e_t; e_t e; struct { logic [69:0] f; } us; logic [69:0] m [2];
  always_comb begin unique case (s) 2'd0: w1 = '1; 2'd1: w1 = 'z; default: w1 = 'x; endcase end
  always_comb w2 = (s == 2'd1) ? 'z : (s == 2'd0) ? '1 : 'x;
  always_latch if (s[0]) w3 = 'z;
  initial begin s = 0; #1 $display("A w1=%h w2=%h", w1, w2); s = 1; us.f = 'z; m = '{default:'1}; m[1] = ('z); for (w4 = '1; 0; ) ; 
    #1 $display("B w1=%h w2=%h w3=%h us=%h m0=%h m1=%h w4=%h", w1, w2, w3, us.f, m[0], m[1], w4); end
  initial #100 $finish;
endmodule
