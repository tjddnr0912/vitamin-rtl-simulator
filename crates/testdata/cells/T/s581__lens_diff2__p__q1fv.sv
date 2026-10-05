package pa; localparam P = 3; localparam N = 5; endpackage
module top;
  import pa::*;
  localparam [64:0] P = 65'h1_0000_0000_0000_0009;
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@ P=%0d Q=%0d w=%0d sh=%0d N=%0d b=%0d", P, Q, w, P >> 60, N, $bits(P));
  if (P > 65'd100) begin : bg initial #2 $display("@ big"); end
endmodule
