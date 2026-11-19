module t;
  localparam logic [127:0] V = 'hz;
  initial begin #1 $display("A v=%h", V); end
  initial #100 $finish;
endmodule
