module top #(parameter type T = logic signed [63:0], parameter T P = -64'sd4);

  localparam R = (P ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
