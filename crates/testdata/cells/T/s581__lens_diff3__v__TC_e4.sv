module top;

  localparam int R = (4'd12 inside {4'b1?00}) ? 10 : 20;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
