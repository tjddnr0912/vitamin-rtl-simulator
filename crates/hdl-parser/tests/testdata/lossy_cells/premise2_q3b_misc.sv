module t; logic [69:0] r1, r2, r3; wire [69:0] w1, w2;
  function automatic logic [69:0] f1(); logic [69:0] v = 'x; return v; endfunction
  function automatic logic [69:0] f2(input int k); f2 = 'z; endfunction
  wire [69:0] na [2]; assign na[1] = 'z;
  assign w1 = f1(); assign w2 = f2(1);
  logic [69:0] cv; assign cv = 'x;
  wire [69:0] pw; assign pw[39:0] = 'z; assign pw[69:40] = '0;
  always_comb r3 = f2(0);
  initial begin #1 $display("A w1=%h w2=%h na1=%h cv=%h pw=%h r3=%h", w1, w2, na[1], cv, pw, r3); end
  final begin r1 = 'x; $display("F r1=%h", r1); end
  initial #100 $finish;
endmodule
