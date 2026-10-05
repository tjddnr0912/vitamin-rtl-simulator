module top;

  localparam logic [7:0] V = 8'b1011_0110;
  localparam logic [3:0] R = V[((4'd15 + 4'd1) ==? 5'b1?000) + 2 : 0];
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
