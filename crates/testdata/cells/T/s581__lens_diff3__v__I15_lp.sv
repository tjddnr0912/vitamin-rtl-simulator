module top;

  localparam R = (4'd12 ==? 4'b1?00 && 4'd3 ==? 4'b0?11);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
