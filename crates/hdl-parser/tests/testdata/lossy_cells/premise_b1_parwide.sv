module t;
  localparam logic [69:0] M = 'z;
  localparam logic [99:0] W = 'x;
  initial begin #1 $display("A m=%h w=%h", M, W); end
  initial #100 $finish;
endmodule
