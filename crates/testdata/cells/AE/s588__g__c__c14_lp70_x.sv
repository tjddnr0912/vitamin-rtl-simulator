module top;
  localparam logic [69:0] P = 70'bx;
  initial begin #1 $display("P=%h", P); $finish; end
  initial #100 $finish;
endmodule
