module t;
  localparam signed A [0:1] = '{4'hF, 4'h1};
  localparam unsigned B [0:1] = '{4'sd8, 4'sd1};
  initial begin #1 $display("A a0=%0d b0=%0d ab=%0d", A[0], B[0], $bits(A[0])); end
  initial #100 $finish;
endmodule
