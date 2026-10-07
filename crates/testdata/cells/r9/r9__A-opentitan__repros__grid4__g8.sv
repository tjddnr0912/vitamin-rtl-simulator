package p; typedef struct packed { logic [1:0] w; logic [1:0] r; } s_t; typedef enum logic [1:0] {A=0,B=1,C=2} e_t; endpackage
module t;
  localparam p::e_t [1:0] L = 4'h9;
  initial begin #1 $display("A localparam_enum=%h", L[1]); $finish; end
endmodule
