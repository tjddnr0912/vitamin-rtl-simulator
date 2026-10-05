module top;

  localparam int R = $bits((4'd15 + 4'd1) ==? 5'b1?000);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
