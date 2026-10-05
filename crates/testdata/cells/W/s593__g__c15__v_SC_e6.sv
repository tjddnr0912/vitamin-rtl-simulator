module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  localparam int R = $clog2((S64N ==? 4'sb1?00) + 3);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
