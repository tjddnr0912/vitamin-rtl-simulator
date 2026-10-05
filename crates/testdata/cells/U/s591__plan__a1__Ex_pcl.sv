module top;
  typedef enum {E0, E1} e_t;
  logic [31:0] k = 1;
  initial #3 case (k) E1: $display("pcl %m hit"); default: $display("pcl %m miss"); endcase
  case (1) E1: begin : gm initial #3 $display("gcl %m hit"); end default: begin : gd initial #3 $display("gcl %m miss"); end endcase
  initial #100 $finish;
endmodule
