module top;
  localparam logic [64:0] P65 = 65'hC;
  typedef enum logic [3:0] {A = (P65 ==? 4'b1?00) ? 4'd5 : 4'd6, B} etc;
  localparam int R = A;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
