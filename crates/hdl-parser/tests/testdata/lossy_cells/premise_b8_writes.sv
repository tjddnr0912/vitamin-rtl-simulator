module t;
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t;
  s_t s; logic [69:0] mem [0:3]; logic [1:0][39:0] pa; logic [69:0] r;
  task automatic tk(input logic [69:0] q); r = q; endtask
  initial begin
    s = '0; pa = '0; s.a = 'x; mem[2] <= 'z; pa[1] = 'z; tk('x);
    #1 $display("A s=%h m=%h pa=%h r=%h", s, mem[2], pa, r);
  end
  initial #100 $finish;
endmodule
