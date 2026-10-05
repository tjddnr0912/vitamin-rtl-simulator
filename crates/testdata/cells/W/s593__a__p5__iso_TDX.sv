`define EX (4'bx100 ==? 4'b1?00)
module t;
  typedef logic [`EX*3:0] TX; TX tx; initial #1 $display("TDX %0d", $bits(tx));
  initial #3 $finish;
endmodule
