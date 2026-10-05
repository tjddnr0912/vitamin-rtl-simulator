module top;
  logic a = 1'b0;
  logic t = 1'b0;
  logic [1:0] y;
  wire [1:0] v;
  assign v = y;
  always_comb y = {a, a};
  always @(t) if ($time > 0) $display("r t=%0t v=%b y=%b", $time, v, y);
  initial begin #1; a = 1'b1; t = 1'b1; #1 $display("e t=%0t v=%b y=%b", $time, v, y); $finish; end
endmodule
