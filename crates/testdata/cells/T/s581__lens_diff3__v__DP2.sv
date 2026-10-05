module sub;
  parameter logic [39:0] P = 0;
  localparam R = (P ==? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub u();
  defparam u.P = 40'hC;
  initial #100 $finish;
endmodule
