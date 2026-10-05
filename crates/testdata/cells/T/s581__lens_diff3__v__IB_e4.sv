module top;

  localparam logic [7:0] V = 8'b1011_0110;
  localparam logic [3:0] R = V[(4'd12 inside {4'b1?00}) * 4 +: 4];
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
