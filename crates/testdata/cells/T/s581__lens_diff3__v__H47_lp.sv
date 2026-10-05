module top;

  localparam R = (4'd5 inside {4'b1?00, 65'd5});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
