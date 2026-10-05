module top;
  for (genvar i = 0; i < 1 + (4'bx100 ==? 4'b1?00); i++) begin : g initial $display("G4 %m"); end
  initial begin   end
endmodule
