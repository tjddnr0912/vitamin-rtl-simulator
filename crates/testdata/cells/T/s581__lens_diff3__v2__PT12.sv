module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  localparam R = (PV ==? 4'sb1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic [63:0]), .PV(64'hFFFF_FFFF_FFFF_FFFC)) u();
  initial #100 $finish;
endmodule
