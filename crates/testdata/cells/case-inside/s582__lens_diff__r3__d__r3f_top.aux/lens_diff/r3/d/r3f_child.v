`timescale 1ns/1ns
module child(output reg [3:0] m);
  reg [3:0] x;
  initial begin x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase end
endmodule
