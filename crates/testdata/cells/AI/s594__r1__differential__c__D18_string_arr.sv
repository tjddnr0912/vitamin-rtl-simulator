module t;
  localparam string SA [2] = '{"ab", "cd"};
  localparam L = (SA[1] == "cd");
  initial $display("R: L=%0d s=%s", L, SA[0]);
  initial #10 $finish;
endmodule
