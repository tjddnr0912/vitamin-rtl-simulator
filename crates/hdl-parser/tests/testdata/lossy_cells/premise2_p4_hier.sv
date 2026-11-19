module sub; logic [69:0] v, v2, v3, up; wire [69:0] w; logic [69:0] m [2];
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t; s_t s;
  initial begin s = '0; t.top_v = 'z; end
endmodule
module t; sub u(); logic [69:0] top_v;
  assign u.w = 'z;
  initial begin u.v = 'x; u.m[1] = 'z; u.v2 <= 'z; u.up = 'x; force u.v3 = 'x;
    #1 $display("A v=%h w=%h m1=%h s=%h", u.v, u.w, u.m[1], u.s); $display("B v2=%h up=%h v3=%h top=%h", u.v2, u.up, u.v3, top_v); end
  initial #100 $finish;
endmodule
