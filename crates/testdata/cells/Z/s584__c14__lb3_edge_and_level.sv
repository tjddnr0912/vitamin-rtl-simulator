module top;
  wire clk;
  wire [1:0] w;
  logic [1:0] y;
  wire [1:0] v;
  assign clk = 1'b1;
  assign w = 2'd2;
  assign v = y;
  always @* y = w;
  always @(posedge clk) $display("P t=%0t v=%b", $time, v);
  always @(w) $display("R t=%0t v=%b", $time, v);
  final $display("F v=%b", v);
endmodule
