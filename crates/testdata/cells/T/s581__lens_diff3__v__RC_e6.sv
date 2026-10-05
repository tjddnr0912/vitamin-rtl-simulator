module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  localparam logic [7:0] R = {(S64N ==? 4'sb1?00) + 1 {4'b1010}};
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
