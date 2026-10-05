localparam N = 3;
localparam [64:0] M = 65'h1_0000_0000_0000_0009;
module top;
  localparam [64:0] N = 65'h1_0000_0000_0000_0007;
  localparam M = 5;
  wire [64:0] wn = N; wire [64:0] wm = M;
  initial #1 $display("@ N=%0d M=%0d wn=%0d wm=%0d", N, M, wn, wm);
  case (N) 65'h1_0000_0000_0000_0007: begin : hn initial #2 $display("@ hitN"); end default: begin : dn initial #2 $display("@ dfltN"); end endcase
  case (M) 5: begin : hm initial #2 $display("@ hitM"); end default: begin : dm initial #2 $display("@ dfltM"); end endcase
endmodule
