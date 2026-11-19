module sub #(parameter type T = logic [3:0]) (); T v; initial #1 $display("S bits=%0d", $bits(v)); endmodule
module t; typedef logic [4'b1x01:0] tx; typedef logic [7:0] t8;
  sub #(.T(tx)) u(); sub #(.T(t8)) u8();
  initial #100 $finish;
endmodule
