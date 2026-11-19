module t; logic [3:0] v1; logic [69:0] v2, v3; logic [7:0] v4; logic signed [69:0] v5;
  initial begin v1 = 4 'b 1x01; v2 = 'B X; v3 = 'h_z; v4 = 8'hXX; v5 = 'sdx; #1 $display("A v1=%b v2=%h v3=%h v4=%h v5=%h", v1, v2, v3, v4, v5); end
  initial #100 $finish;
endmodule
