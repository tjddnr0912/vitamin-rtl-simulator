module t #(parameter type T = logic [7:0]);
  localparam T X = -4;
  typedef struct packed { logic [X+8:0] a; } s_t;
  initial begin $display("sb=%0d X=%0d", $bits(s_t), X); #1 $finish; end
endmodule
