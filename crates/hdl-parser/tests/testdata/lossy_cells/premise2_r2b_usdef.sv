module t;
  typedef struct { logic [69:0] a = 'x; int b; } us_t; us_t us;
  struct { logic [69:0] m = 'z; logic [7:0] n; } us2;
  initial begin #1 $display("A a=%h m=%h", us.a, us2.m); end
  initial #100 $finish;
endmodule
