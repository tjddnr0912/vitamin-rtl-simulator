module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  localparam T LV = 40'h10_0000_000C;
  localparam R = (PV ==? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic [39:0]), .PV(40'h10_0000_000C)) u();
  initial #100 $finish;
endmodule
