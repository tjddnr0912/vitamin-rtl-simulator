let LU = 'x;
module t; logic [69:0] w; initial begin w = LU; #1 $display("A w=%h", w); end initial #100 $finish; endmodule
