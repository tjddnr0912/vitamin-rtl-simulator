module t;
  initial begin : b
    localparam unsigned U = 4'sd8;
    #1 $display("A u=%0d gt=%0d", U, U > 0);
  end
  initial #100 $finish;
endmodule
