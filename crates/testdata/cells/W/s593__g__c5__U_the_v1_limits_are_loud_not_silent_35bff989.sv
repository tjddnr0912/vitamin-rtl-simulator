module tb; typedef logic [3:0][4:0] pt; localparam pt P = 20'h12345;
  initial begin $display("DIGEST=%h %h", P, P[1]); #1 $finish; end endmodule