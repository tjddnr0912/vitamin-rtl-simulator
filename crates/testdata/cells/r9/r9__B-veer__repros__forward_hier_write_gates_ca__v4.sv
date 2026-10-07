module slv (output reg rdy, output logic [4:0] idx);
  assign rdy = 1'b1;
  assign idx = 5'd3;
endmodule
module top;
  wire r;
  logic clk = 0;
  slv u (.rdy(r), .idx());
  always @(posedge clk) begin
    top.g[0][3] = 32'd7;               // self-hierarchical write, index read through another hierarchical name
  end
  initial begin #1 clk = 1; #1 $display("r=%b g03=%0d", r, g[0][3]); $finish; end
  bit [31:0] [31:0] g [1];
endmodule
