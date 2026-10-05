package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; s_t V = '{a:4'h1, b:4'h2}; endpackage
module tb; logic [7:0] V [0:1]; import p::*;
  initial begin V = '{8'h12, 8'h34}; #1 $display("DIGEST=%h %h", V[0], V[1]); #1 $finish; end
endmodule