module top;
  logic [3:0] a = 4'd3;
  logic [3:0] y, z;
  always_comb z = y + 4'd1;
  always_comb y = a + 4'd1;
  final $display("FIN y=%0d z=%0d", y, z);
endmodule
