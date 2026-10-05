`timescale 1ns/1ns
module top;
  logic [7:0] v;
  logic signed [31:0] d;
  logic [31:0] r;
  initial begin
    v = 8'hF0;
    d = signed'(v);
    r = unsigned'(v);
    $display("d=%0d r=%0d neg=%0d", d, r, (signed'(v) - 1) < 0);
  end
  initial #10 $finish;
endmodule
