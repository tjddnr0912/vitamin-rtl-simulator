module top;
  localparam int N2 = 2;
  localparam logic [39:0] PA [2] = '{40'h10_0000_000C, 40'h0};
  localparam R = ((N2 == 2 ? PA[0] : 40'd0) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
