module top;
  logic [7:0] x = 8'hA5;
  wire [3:0] y = x[0 +: 2 + (4'bx100 ==? 4'b1?00)];
  initial #1 $display("G2b y=%b", y);
endmodule
