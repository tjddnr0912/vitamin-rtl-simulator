package p; typedef struct packed { logic [1:0] w; logic [1:0] r; } s_t; typedef enum logic [1:0] {A=0,B=1,C=2} e_t; endpackage
module t;
  parameter p::e_t [1:0][1:0] M = 8'h96;
  initial begin #1 $display("A param_enum_md=%h", M[1][0]); $finish; end
endmodule
