`define DECL(n) reg [3:0] n;
`define SET(n, v) n = v;
module top;
  `DECL(inside)
  reg [3:0] x; integer m;
  initial begin `SET(inside, 4'b0110) x = 4'd2;
    case (x) inside[1:0]: m = 1; default: m = 0; endcase
    $display("r3b macro-arg m=%0d", m); #1 $finish; end
endmodule
