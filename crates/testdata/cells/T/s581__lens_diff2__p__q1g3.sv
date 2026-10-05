package pa; localparam P = 3; localparam N = 5; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module rd(input logic [64:0] pi); initial #2 $display("@port pi=%0d hier=%0d", pi, top.P); endmodule
module top;
  import pb::P;
  import pa::*;
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  wire [4:0] ps = P[64:60];
  logic [64:0] x;
  rd u(.pi(P));
  initial begin x = P + 65'd2; #1 $display("@ P=%0d Q=%0d w=%0d ps=%0d x=%0d N=%0d b=%0d", P, Q, w, ps, x, N, $bits(P)); end
  if (P > 65'd100) begin : bg initial #3 $display("@ big"); end else begin : sm initial #3 $display("@ small"); end
endmodule
