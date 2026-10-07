module c (input logic [3:0] i, output logic [3:0] q); assign q = i + 4'd1; endmodule
module t;
  typedef struct packed { logic [3:0] q; } f_t;
  typedef struct packed { f_t a; f_t b; } e_t;
  typedef struct packed { e_t m1; e_t m0; } s_t;
  s_t s; e_t [1:0] v;
  c u0 (.i(4'd2), .q(s.m1.b.q));
  c u1 (.i(4'd6), .q(v[1].b.q));
  initial begin #1 $display("A s=%h v=%h", s.m1.b.q, v[1].b.q); $finish; end
endmodule
