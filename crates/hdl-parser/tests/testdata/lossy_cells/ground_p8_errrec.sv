module t;
  localparam logic [3:0] Q = 4'bx;
  logic [8 / Q : 0] w;
  initial begin #1 $display("A bw=%0d", $bits(w)); end
  initial #100 $finish;
endmodule
