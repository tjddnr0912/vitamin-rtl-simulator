module t; localparam unsigned [3:0] U = 4'sd8; localparam signed [3:0] S = 4'hF;
  initial begin #1 $display("A u=%0d gt=%0d s=%0d lt=%0d", U, (U > 0), S, (S < 0)); end
  initial #100 $finish;
endmodule
