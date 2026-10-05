module top;
  localparam logic [39:0] PA [2] = '{40'h10_0000_000C, 40'h0};
  localparam R = ($size(PA) ==? 2'b1?);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
