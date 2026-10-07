module dff #(parameter W = 4) (input logic clk, input logic [W-1:0] d, output logic [W-1:0] q);
  always_ff @(posedge clk) q <= d;
endmodule
module top;
  logic clk = 0;
  logic [1:0][15:2] r;                                     // packed 2-D whose element range has a non-zero LSB
  logic [1:0][36:13] w;
  dff #(14) f0 (.clk(clk), .d(14'h5), .q(r[0][15:2]));    // output actual: element + part-select
  dff #(14) f1 (.clk(clk), .d(14'h6), .q(r[1][15:2]));
  for (genvar i = 0; i < 2; i++) begin : g
    assign w[i][31:13] = 19'(i + 1);                       // continuous assign: element + part-select
    assign w[i][36:32] = 5'(i + 7);
  end
  initial begin #1 clk = 1; #1 $display("r=%h w=%h", r, w); $finish; end
endmodule
