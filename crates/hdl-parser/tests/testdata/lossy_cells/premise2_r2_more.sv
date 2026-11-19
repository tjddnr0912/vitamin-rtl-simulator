module t; logic [39:0] a; logic [29:0] b; logic [34:0] c, d;
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t; s_t qs[$]; s_t as[int];
  initial begin
    qs.push_back('0); qs[0].a = 'x; as[5] = '0; as[5].b = 'z;
    #1 $display("A qs0=%h as5=%h", qs[0], as[5]);
  end
  initial #100 $finish;
endmodule
