module sub;
  parameter logic signed [63:0] P = 0;
  localparam R = (P ==? 4'sb1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub u();
  defparam u.P = -4;
  initial #100 $finish;
endmodule
