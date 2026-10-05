package pa; localparam P = 3; localparam N = 5; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  import pb::P;
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@ P=%0d Q=%0d w=%0d sh=%0d N=%0d b=%0d", P, Q, w, P >> 60, N, $bits(P));
  case (P)
    65'h1_0000_0000_0000_0009: begin : hw initial #2 $display("@ hitw"); end
    3: begin : h3 initial #2 $display("@ hit3"); end
    default: begin : dd initial #2 $display("@ dflt"); end
  endcase
  if (P > 65'd100) begin : big initial #2 $display("@ big"); end
endmodule
