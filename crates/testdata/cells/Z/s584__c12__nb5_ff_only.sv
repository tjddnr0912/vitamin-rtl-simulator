module top;
  wire clk;
  wire [1:0] w;
  logic [1:0] q, y;
  wire [1:0] v;
  assign clk = 1'b1;
  assign w = 2'd1;
  assign v = y;
  always_comb y = w;
  always @(posedge clk) begin q <= v; $display("P t=%0t v=%b y=%b", $time, v, y); end
  final $display("F q=%b", q);
endmodule
