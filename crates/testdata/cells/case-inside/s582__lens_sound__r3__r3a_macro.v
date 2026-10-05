`define INS inside
module top;
  reg [3:0] `INS; reg [3:0] x; integer m;
  initial begin `INS = 4'b0110; x = 4'd2;
    case (x) inside[1:0]: m = 1; default: m = 0; endcase
    $display("r3a macro-name m=%0d", m); #1 $finish; end
endmodule
