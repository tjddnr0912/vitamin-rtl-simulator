module top;
  wire [1:0] w;
  logic [1:0] y;
  wire [1:0] v;
  assign w = 2'd1;
  assign v = y;
  always @* y = w;
  always @(w) $display("r t=%0t w=%b v=%b y=%b", $time, w, v, y);
  final $display("F v=%b y=%b", v, y);
endmodule
