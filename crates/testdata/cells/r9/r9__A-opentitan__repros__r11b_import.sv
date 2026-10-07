package mp;
  parameter int W = 4;
  typedef enum logic [W-1:0] { T = 4'h6, F = 4'h9 } m_t;
  function automatic logic inv(m_t v); return ~((v == T) || (v == F)); endfunction
endpackage
package tp;
  typedef struct packed { logic [2:0] r; mp::m_t it; } u_t;
  function automatic logic chk(u_t u); logic e; e = mp::inv(u.it); return e; endfunction
endpackage
module t; import tp::*;
  tp::u_t u; logic o;
  assign o = chk(u);
  initial begin u = 7'b000_0110; #1 $display("A o=%b", o); u = 7'b000_0111; #1 $display("A o=%b", o); $finish; end
endmodule
