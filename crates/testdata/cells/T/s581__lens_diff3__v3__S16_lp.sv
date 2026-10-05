module top;
  localparam integer PI = -4;
  localparam R = (PI ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
