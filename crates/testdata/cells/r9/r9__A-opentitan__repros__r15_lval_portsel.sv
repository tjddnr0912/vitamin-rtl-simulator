module c (input logic [3:0] i, output logic [3:0] q); assign q = i + 4'd1; endmodule
module t;
  typedef struct packed { logic [3:0] q; } e_t;
  typedef struct packed { logic [7:0] m; logic x; } s_t;
  s_t s; e_t [1:0] v;
  c u0 (.i(4'd2), .q(s.m[7:4]));
  c u1 (.i(4'd6), .q(v[1].q));
  initial begin #1 $display("A s.m=%h v=%h", s.m[7:4], v[1]); $finish; end
endmodule
