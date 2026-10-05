module top;
  logic [3:0] a, z;
  initial #0 for (int unsigned i = 0; i < 4; i++) z[i] = a[i];
  initial begin a = 4'h5; #1 $display("z=%h", z); $finish; end
endmodule
