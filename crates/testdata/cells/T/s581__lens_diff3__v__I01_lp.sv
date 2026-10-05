module top;

  localparam R = ((4'd12 inside {4'b1?00}) inside {1'b1});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
