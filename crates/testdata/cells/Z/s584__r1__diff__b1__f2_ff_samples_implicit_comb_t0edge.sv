// child always_ff samples, on a time-0 posedge (x->1), a CA of an implicit-only comb
module ch(input logic clk, input logic [1:0] d, output logic [1:0] q);
  always_ff @(posedge clk) q <= d;
endmodule
module top;
  logic clk;
  logic [1:0] c = 2'd2;
  logic [1:0] y;
  always_comb y = c;
  wire [1:0] w = y;
  wire [1:0] q;
  ch u(.clk(clk), .d(w), .q(q));
  initial begin clk = 1'b1; #1 $display("t=%0t q=%b w=%b", $time, q, w); end
  initial #10 $finish;
endmodule
