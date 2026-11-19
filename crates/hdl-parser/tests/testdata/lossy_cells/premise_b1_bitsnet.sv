module t; wire [69:0] n = 'z; logic c; wire [69:0] m; assign m = c ? 'x : 70'h0;
  initial begin c = 1; #1 $display("A n=%h m=%h b1=%0d b2=%0d", n, m, $bits('x), $bits({'z})); end
  initial #100 $finish;
endmodule
