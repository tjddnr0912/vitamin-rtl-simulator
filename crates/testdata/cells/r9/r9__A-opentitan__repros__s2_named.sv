module t;
  typedef struct packed { logic [1:0] q; } a_t;
  typedef struct packed { logic q; logic qe; } b_t;
  typedef struct packed { a_t a; b_t b; } r_t;
  r_t r;
  initial begin r = 5'b10110; #1 $display("A a.q=%b b.q=%b b.qe=%b", r.a.q, r.b.q, r.b.qe); $finish; end
endmodule
