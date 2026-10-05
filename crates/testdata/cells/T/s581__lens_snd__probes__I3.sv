module top;
  parameter A = 4'h1;
  localparam W = A << 4;
  localparam L = (W ==? 8'b0001_????);
  initial $display("I3 W=%0d L=%0d", W, L);
endmodule
