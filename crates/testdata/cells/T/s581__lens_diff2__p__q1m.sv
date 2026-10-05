package pk;
  localparam [64:0] W = 65'h1_0000_0000_0000_0009;
  localparam N = 3;
  function automatic int pf(input int a); int W; W = a + N; return W; endfunction
  function automatic logic [64:0] pw(); return W; endfunction
endpackage
module top;
  import pk::*;
  localparam int R = pf(5);
  localparam [64:0] Q = pw();
  initial #1 $display("@ W=%0d N=%0d R=%0d Q=%0d pf=%0d pw=%0d", W, N, R, Q, pf(7), pw());
  case (W) 65'h1_0000_0000_0000_0009: begin : hw initial #2 $display("@ hitw"); end default: begin : dd initial #2 $display("@ dflt"); end endcase
endmodule
