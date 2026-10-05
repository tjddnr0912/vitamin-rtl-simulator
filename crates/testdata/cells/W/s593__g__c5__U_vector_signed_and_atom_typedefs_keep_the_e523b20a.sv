module tb; typedef logic [5:0] u;
  localparam u A = 6'd1, B = 6'd2, C = 6'd63; localparam u D = C + 1;
  initial begin $display("DIGEST=%0d %0d %0d %0d", A, B, C, D); #1 $finish; end
endmodule