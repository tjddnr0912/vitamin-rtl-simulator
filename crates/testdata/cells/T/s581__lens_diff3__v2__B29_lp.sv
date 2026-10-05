module top;
  localparam logic [69:0] PA70 [2] = '{70'hC, 70'h0};
  localparam R = (PA70[0] ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
