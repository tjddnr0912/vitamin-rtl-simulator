module t;
  typedef enum logic [69:0] { E0 = 70'd0, E1 = 70'd1 } e_t;
  typedef struct { logic [69:0] a; int b; } us_t;
  e_t e; real r; string s; us_t us; logic [69:0] ua [2]; logic [69:0] d [];
  initial begin
    e = 'x;
    r = 'x;
    s = 'z;
    us = 'x;
    ua = 'z;
    d = 'x;
    #1 $display("A e=%h r=%f s=%s us=%h ua0=%h", e, r, s, us.a, ua[0]);
  end
  initial #100 $finish;
endmodule
