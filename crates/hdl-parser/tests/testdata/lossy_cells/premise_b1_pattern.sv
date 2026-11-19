module t;
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t;
  s_t s1, s2; logic [69:0] arr [0:1]; logic [69:0] d [0:1];
  initial begin
    s1 = '{a: 'x, b: '1}; s2 = '{default: 'z};
    arr = '{'x, 'z}; d = '{default: 'x};
    #1 $display("A s1=%h s2=%h a0=%h a1=%h d1=%h", s1, s2, arr[0], arr[1], d[1]);
  end
  initial #100 $finish;
endmodule
