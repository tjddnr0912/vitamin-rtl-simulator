module top;
  localparam logic [3:0] P = 4'bxxxx | 4'b0011;
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
