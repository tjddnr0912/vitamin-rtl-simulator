module sub #(parameter type T = logic signed [7:0]);
  localparam T AT [0:1] = '{8'hFC, 8'hFC};
  localparam int L = (AT[0] + 8'sd0) < 0;
  localparam int M = ((AT[0] + 8'd4) == 0);
  initial $display("%m L=%0d M=%0d", L, M);
endmodule
module t;
  sub u1();
  sub #(.T(logic [7:0])) u2();
  sub #(.T(logic signed [15:0])) u3();
  initial #100 $finish;
endmodule
