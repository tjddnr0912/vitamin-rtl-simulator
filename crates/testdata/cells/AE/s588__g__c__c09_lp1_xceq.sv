module top;
  localparam logic P = (4'bxxxx === 4'bxxxx);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
