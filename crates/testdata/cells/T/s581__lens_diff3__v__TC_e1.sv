module top;

  localparam int R = ((4'd15 + 4'd1) ==? 5'b1?000) ? 10 : 20;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
