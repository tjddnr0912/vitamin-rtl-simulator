package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t V = '{a:4'h1, b:4'h2}; endpackage
module tb; import p::*; logic [7:0] V [0:1];
  initial begin V = '{8'hAA, 8'hBB}; #1 $display("DIGEST=%h %h", V[0], V[1]); #1 $finish; end
endmodule