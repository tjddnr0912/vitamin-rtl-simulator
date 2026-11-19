package p;
  localparam unsigned U = 4'sd8;
endpackage
module t;
  localparam V = 4'd8;
  initial begin #1 $display("A v=%0d", V); end
  initial #100 $finish;
endmodule
