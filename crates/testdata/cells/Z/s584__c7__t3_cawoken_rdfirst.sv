module top;
  logic a = 1'b0;
  logic t = 1'b0, t2 = 1'b0;
  wire [1:0] w;
  logic [1:0] y;
  wire [1:0] v;
  assign w = {a, a};
  assign v = y;
  always @(t) t2 = 1'b1;
  always @(t2) $display("r t=%0t v=%b y=%b", $time, v, y);
  always_comb y = w;
  initial begin #1; a = 1'b1; t = 1'b1; #1 $display("e t=%0t v=%b y=%b", $time, v, y); $finish; end
endmodule
