module t;
  parameter L3 = 4'd15;
  typedef logic [L3 - 5'sd13 : 0] t_t;
  localparam M = $bits(t_t) - 8;
  logic [8 / (M < 0) : 0] w;
  initial begin #1 $display("A m=%0d bw=%0d", M, $bits(w)); end
  initial #100 $finish;
endmodule
