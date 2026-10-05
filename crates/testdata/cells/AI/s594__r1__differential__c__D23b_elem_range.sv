module t;
  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
  localparam B1 = $bits(AS[0]);
  logic [AS[1] + {4{1'b0}} : 0] w;
  logic [(AS[0] + 8'd0) - 250 : 0] u;
  initial $display("R: B1=%0d wb=%0d ub=%0d", B1, $bits(w), $bits(u));
  initial #10 $finish;
endmodule
