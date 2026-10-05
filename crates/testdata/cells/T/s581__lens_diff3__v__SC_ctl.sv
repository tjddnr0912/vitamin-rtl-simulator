module top;

  localparam int R = $clog2((4'd12 == 4'd12) + 3);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
