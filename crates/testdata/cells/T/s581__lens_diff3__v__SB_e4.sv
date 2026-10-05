module top;

  localparam int R = $bits(4'd12 inside {4'b1?00});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
