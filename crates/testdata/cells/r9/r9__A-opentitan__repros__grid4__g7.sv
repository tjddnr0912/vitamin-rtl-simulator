package p; typedef struct packed { logic [1:0] w; logic [1:0] r; } s_t; typedef enum logic [1:0] {A=0,B=1,C=2} e_t; endpackage
module t;
  localparam p::s_t [1:0] L = 8'hA5;
  initial begin #1 $display("A localparam_struct=%h", L[1].w); $finish; end
endmodule
