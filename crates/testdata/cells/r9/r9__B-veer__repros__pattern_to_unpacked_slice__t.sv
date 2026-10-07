module top;
  logic       v [1:0];
  logic [4:0] d [1:0];
  logic clk = 0;
  always @(posedge clk) begin
    v[1:0] <= '{1'b1, 1'b0};                // assignment pattern to a slice (here the whole range) of an unpacked array
    d[1:0] <= '{5'd3, 5'd7};
  end
  initial begin #1 clk = 1; #1 $display("v1=%b v0=%b d1=%0d d0=%0d", v[1], v[0], d[1], d[0]); $finish; end
endmodule
