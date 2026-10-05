module top;
  localparam logic [3:0] P = 4'bxxxx;
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
