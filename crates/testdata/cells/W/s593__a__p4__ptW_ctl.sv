module t #(parameter W = 8);
  localparam logic [W-1:0] X = -4;
  typedef struct packed { logic [X+8:0] a; } s_t;
  initial begin $display("sb=%0d X=%0d", $bits(s_t), X); #1 $finish; end
endmodule
