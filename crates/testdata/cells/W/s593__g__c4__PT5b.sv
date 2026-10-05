module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  localparam int R = $bits(PV);
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
  initial #100 $finish;
endmodule
