package u; function automatic integer bits_for(integer n); return $clog2(n + 1); endfunction endpackage
package p;
  parameter int unsigned N = 5;
  parameter int unsigned L = u::bits_for(N);
  typedef logic [L-1:0] sel_t;
  typedef struct packed { sel_t s; logic e; } r_t;
endpackage
module t;
  p::r_t r;
  initial begin r = 4'hD; #1 $display("A s=%h e=%b bits=%0d L=%0d", r.s, r.e, $bits(r), p::L); $finish; end
endmodule
