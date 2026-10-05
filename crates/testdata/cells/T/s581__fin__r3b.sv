module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  localparam R = (PV ==? 4'sb1?00);
  initial $display("R=%0d lt0=%0d", R, PV < 0);
endmodule
module t;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
endmodule
