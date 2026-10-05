package p1; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; endpackage
package p2; import p1::*; s_t V; localparam s_t Q = '{4'h1, 4'h2}; endpackage
module tb; typedef struct packed { logic [11:0] a; logic [3:0] b; } s_t; import p2::*;
  initial begin V = 8'h12; $display("DIGEST=%h %h %h %h", V.a, V.b, Q.a, Q.b); #1 $finish; end
endmodule