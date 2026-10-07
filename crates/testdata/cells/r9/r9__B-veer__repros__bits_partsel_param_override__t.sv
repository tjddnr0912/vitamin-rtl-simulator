module dff #(parameter WIDTH = 1) (input logic clk, input logic [WIDTH-1:0] din, output logic [WIDTH-1:0] dout);
  always_ff @(posedge clk) dout <= din;
  initial $display("WIDTH=%0d", WIDTH);
endmodule
module top;
  logic clk = 0;
  logic [31:0] addr = 32'h30;
  logic [3:0]  q;
  dff #(2*$bits(addr[5:4])) f (.clk(clk), .din({addr[5:4], addr[5:4]}), .dout(q));   // $bits of a part-select as an override
  initial begin #1 clk = 1; #1 $display("q=%h", q); $finish; end
endmodule
