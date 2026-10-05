module top;

  localparam logic [7:0] R = {((4'd15 + 4'd1) ==? 5'b1?000) + 1 {4'b1010}};
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
