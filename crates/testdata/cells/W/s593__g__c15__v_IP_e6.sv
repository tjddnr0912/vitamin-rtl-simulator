module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  localparam logic [7:0] V = 8'b1011_0110;
  localparam logic [1:0] R = V[0 +: (S64N ==? 4'sb1?00) + 1];
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
