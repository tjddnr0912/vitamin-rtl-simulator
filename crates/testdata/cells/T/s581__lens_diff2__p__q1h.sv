module top;
  localparam [64:0] P = 65'h1_0000_0000_0000_0009;
  function automatic int f(input int P); return P + 1; endfunction
  function automatic int g(input int a); int P; P = a * 2; return P; endfunction
  localparam int R = f(5);
  localparam int S = g(5);
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@ P=%0d R=%0d S=%0d Q=%0d w=%0d f7=%0d g7=%0d", P, R, S, Q, w, f(7), g(7));
  case (P)
    65'h1_0000_0000_0000_0009: begin : hw initial #2 $display("@ hitw"); end
    default: begin : dd initial #2 $display("@ dflt"); end
  endcase
  if (P > 65'd100) begin : big initial #2 $display("@ big"); end
endmodule
