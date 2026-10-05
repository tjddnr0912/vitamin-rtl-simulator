module top;
  localparam P = 3;
  if (1) begin : b
    localparam [64:0] P = 65'h1_0000_0000_0000_0009;
    initial #1 $display("cmp gt=%0d eq3=%0d", (P > 65'd100), (P == 3));
  end
  initial #100 $finish;
endmodule
