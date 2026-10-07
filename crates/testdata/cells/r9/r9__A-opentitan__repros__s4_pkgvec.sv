package p;
  localparam int N = 3;
  typedef struct packed { logic [1:0] w; logic [1:0] r; } pol_t;
  typedef pol_t [N-1:0] vec_t;
  parameter vec_t DEF = '0;
endpackage
module t;
  p::vec_t v;
  initial begin v = 12'hA5C; #1 $display("A v1.w=%b v2.r=%b bits=%0d def=%h", v[1].w, v[2].r, $bits(v), p::DEF); $finish; end
endmodule
