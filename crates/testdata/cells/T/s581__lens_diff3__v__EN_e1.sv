module top;

  typedef enum logic [3:0] {A = ((4'd15 + 4'd1) ==? 5'b1?000) ? 4'd5 : 4'd6, B} etc;
  localparam int R = A;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
