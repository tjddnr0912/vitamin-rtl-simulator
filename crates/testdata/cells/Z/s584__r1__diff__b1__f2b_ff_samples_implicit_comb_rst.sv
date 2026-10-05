// async-reset flop beside an implicit-only comb; posedge clk at t0 samples comb output directly
module top;
  logic clk, rst_n;
  logic [3:0] base = 4'd5;
  logic [3:0] d, q;
  always_comb d = base + 4'd1;
  always_ff @(posedge clk or negedge rst_n)
    if (!rst_n) q <= 4'd0; else q <= d;
  initial begin rst_n = 1'b1; clk = 1'b1; #1 $display("t=%0t q=%0d d=%0d", $time, q, d); end
  initial #10 $finish;
endmodule
