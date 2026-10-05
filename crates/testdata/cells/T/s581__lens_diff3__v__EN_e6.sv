module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  typedef enum logic [3:0] {A = (S64N ==? 4'sb1?00) ? 4'd5 : 4'd6, B} etc;
  localparam int R = A;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
