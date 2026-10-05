module top;

  localparam R = ((4'd12 ==? 4'b1?00) ==? 1'b?);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
