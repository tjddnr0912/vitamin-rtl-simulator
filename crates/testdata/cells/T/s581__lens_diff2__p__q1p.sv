package pa; localparam R = 1.5; localparam S = "ab"; localparam N = 5; endpackage
package pb; localparam [64:0] R = 65'h1_0000_0000_0000_0007; localparam [64:0] S = 65'h1_0000_0000_0000_0005; endpackage
module top;
  import pa::*;
  import pb::R;
  import pb::S;
  localparam [64:0] QR = R + 65'd1;
  localparam [64:0] QS = S + 65'd1;
  wire [64:0] wr = R;
  wire [64:0] ws = S;
  initial #1 $display("@ R=%0d QR=%0d wr=%0d S=%0d QS=%0d ws=%0d N=%0d", R, QR, wr, S, QS, ws, N);
endmodule
