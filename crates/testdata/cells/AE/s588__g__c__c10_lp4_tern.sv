module top;
  localparam logic [3:0] P = 1'bx ? 4'd1 : 4'd2;
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
