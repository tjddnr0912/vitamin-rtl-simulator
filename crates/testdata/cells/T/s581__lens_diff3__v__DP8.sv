module sub;
  parameter P = 0;
  localparam R = (P ==? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub u();
  defparam u.P = 65'hC;
  initial #100 $finish;
endmodule
