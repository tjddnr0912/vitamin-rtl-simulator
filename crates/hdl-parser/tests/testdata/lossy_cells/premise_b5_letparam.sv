module t; let LX = 'x; let LB = 8'b1x01_0000;
  localparam logic [69:0] P = LX; localparam logic [7:0] Q = LB;
  initial begin #1 $display("A p=%h q=%b", P, Q); end
  initial #100 $finish;
endmodule
