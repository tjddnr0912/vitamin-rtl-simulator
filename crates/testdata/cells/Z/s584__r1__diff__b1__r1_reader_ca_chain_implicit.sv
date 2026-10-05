module top;
  logic [1:0] c = 2'd1;
  logic [1:0] y;
  always_comb y = c + 2'd1;
  wire [1:0] v = y;
  logic t = 1'b0;
  wire t1 = t;
  wire t2 = t1;
  always @(t2) $display("R t=%0t v=%b y=%b", $time, v, y);
  initial t = 1'b1;
  initial #5 $display("E v=%b y=%b", v, y);
  initial #10 $finish;
endmodule
