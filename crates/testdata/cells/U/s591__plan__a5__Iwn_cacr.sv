package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pb::*;
  import pa::P;
  sacr #(.N(P)) u8 (.a('0));
  initial #100 $finish;
endmodule
module sacr #(parameter N = 0) (input logic [N:0] a);
  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; 3: fc = 6; default: fc = 7; endcase endfunction
  wire [7:0] r = {fc(N){1'b1}};
  initial #3 $display("acr %m r=%b b=%0d", r, $bits(a));
endmodule
