module m #(parameter int N = 4) (input logic [Nope1-1:0] a); localparam logic [Nope2-1:0] Y = 4'h9; logic [Nope3-1:0] v; initial begin v = '1; $display("DIGEST=%0d %0d %0d %0d", $bits(a), $bits(Y), $bits(v), $bits(m.a)); end endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #1 $finish; end endmodule
