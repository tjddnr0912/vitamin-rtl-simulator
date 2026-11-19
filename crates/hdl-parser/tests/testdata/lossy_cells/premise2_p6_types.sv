module sub #(parameter int W = 8, type T = logic [7:0]); logic [W-1:0] v = 'x; T tv; logic [W-1:0] v2; initial begin tv = 'z; v2 = 'z; end endmodule
module t;
  sub #(.W(70), .T(logic [69:0])) u();
  logic [69:0] q[$]; logic [69:0] aa[string];
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t; s_t ps;
  typedef union packed { logic [69:0] x; s_t y; } u_t; u_t pu;
  typedef struct { logic [69:0] a; int b; } us_t; us_t us;
  logic [1:0][39:0] pa; logic [69:0] g1;
  for (genvar gi = 0; gi < 1; gi++) begin : g logic [69:0] gv = 'z; logic [69:0] gv2; initial gv2 = 'x; end
  initial begin q.push_back(70'h0); q[0] = 'x; aa["k"] = 'z; ps = 'x; pu = 'z; us.a = 'x; pa[1:0] = 'x;
    #1 $display("A uv=%h utv=%h uv2=%h q0=%h aa=%h", u.v, u.tv, u.v2, q[0], aa["k"]); $display("B ps=%h pu=%h usa=%h pa=%h gv=%h gv2=%h", ps, pu, us.a, pa, g[0].gv, g[0].gv2); end
  initial #100 $finish;
endmodule
