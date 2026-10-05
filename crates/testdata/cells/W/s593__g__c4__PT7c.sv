module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  localparam R = (PV < 0);
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic signed [7:0]), .PV(-8'sd4)) u();
  initial #100 $finish;
endmodule
