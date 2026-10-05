module sub #(parameter logic signed [7:0] AS [2] = '{8'sd1, 8'sd2}) ();
  localparam L = AS[0] + {4{1'b0}};
  localparam K = (AS[0] < 8'sd0);
  initial $display("R: L=%0d B=%0d K=%0d", L, $bits(L), K);
endmodule
module t;
  sub #(.AS('{-8'sd4, 8'sd3})) u();
  initial #10 $finish;
endmodule
