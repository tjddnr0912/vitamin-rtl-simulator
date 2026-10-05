module top;

  localparam R = ($signed(4'b1100) ==? 8'sb1111_1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
