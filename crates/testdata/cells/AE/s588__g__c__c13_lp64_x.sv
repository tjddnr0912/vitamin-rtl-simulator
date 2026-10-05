module top;
  localparam logic [63:0] P = 64'bx;
  initial begin #1 $display("P=%h", P); $finish; end
  initial #100 $finish;
endmodule
