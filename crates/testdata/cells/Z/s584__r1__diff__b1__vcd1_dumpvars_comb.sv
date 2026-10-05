module top;
  logic [1:0] c = 2'd1;
  logic [1:0] y, z;
  logic s;
  always_comb y = c + 2'd1;
  always_comb z = y ^ {2{s}};
  initial begin $dumpfile("iv.vcd"); $dumpvars(0, top); s = 1'b0; #5 s = 1'b1; #5 $finish; end
endmodule
