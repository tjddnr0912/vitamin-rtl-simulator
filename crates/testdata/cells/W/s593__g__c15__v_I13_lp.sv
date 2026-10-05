module top;

  localparam R = (~4'd3 ==? 8'b1111_1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
