module dff #(parameter W = 4) (input logic clk, input logic [W-1:0] d, output logic [W-1:0] q);
  always_ff @(posedge clk) q <= d;
endmodule
module top;
  localparam W = 8;
  logic clk = 0;
  logic [1:0][7:0] a, b, c;
  // A: genvar index + parameter range, output port actual
  for (genvar i = 0; i < 2; i++) begin : g
    dff #(W) f (.clk(clk), .d(8'h30 + 8'(i)), .q(a[i][W-1:0]));
  end
  // B: procedural loop-variable index + literal part-select
  always_comb begin
    b = '0;
    for (int i = 0; i < 2; i++) b[i][7:0] |= 8'h11 << i;
  end
  // C: genvar index + literal range, continuous assign
  for (genvar i = 0; i < 2; i++) begin : h
    assign c[i][3:0] = 4'(i + 5);
    assign c[i][7:4] = 4'(i + 9);
  end
  initial begin
    #1 clk = 1; #1;
    $display("a=%h b=%h c=%h", a, b, c);
    $finish;
  end
endmodule
