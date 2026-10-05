module top;
  localparam logic [3:0] P = 4'b10x1;
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
