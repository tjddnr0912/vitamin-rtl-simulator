module m #(parameter W = 8);
  localparam logic [W-1:0] X = -4;
  typedef struct packed { logic [X+8:0] a; } s_t;
  initial begin $display("%m sb=%0d X=%0d", $bits(s_t), X); end
endmodule
module t;
  m #(.W(16)) u16();
  m u8();
  initial #1 $finish;
endmodule
