module t;
  localparam U = 4'd8;
  initial begin #1 $display("A u=%0d gt=%b", U, U > 4'sd0); end
  initial #100 $finish;
endmodule
