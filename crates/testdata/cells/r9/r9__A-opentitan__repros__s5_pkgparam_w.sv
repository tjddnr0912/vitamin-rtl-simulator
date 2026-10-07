package tp; localparam int AUW = 23; endpackage
package mp; parameter int MW = 4; endpackage
package p;
  parameter int H = 7;
  parameter int RW = tp::AUW - mp::MW - H - 7;
  typedef struct packed { logic [RW-1:0] rsvd; logic [3:0] it; } u_t;
endpackage
module t;
  p::u_t u;
  initial begin u = 9'h1A5; #1 $display("A rsvd=%h it=%h bits=%0d", u.rsvd, u.it, $bits(u)); $finish; end
endmodule
