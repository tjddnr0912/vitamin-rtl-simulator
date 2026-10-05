module top;
  wire [1:0] w;
  logic [1:0] s;
  logic [1:0] y;
  wire [1:0] v;
  logic t;
  assign w = 2'd1;
  assign v = y;
  always_comb y = w ^ s;
  initial begin s = 2'd3; t = 1'b1; end
  always @(t) $display("b2 t=%0t v=%b y=%b", $time, v, y);
  initial #1 begin $display("e t=%0t v=%b y=%b", $time, v, y); $finish; end
endmodule
