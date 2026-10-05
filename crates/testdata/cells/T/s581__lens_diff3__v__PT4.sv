module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  localparam T LV = 40'h10_0000_000C;
  localparam R = (LV ==? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
