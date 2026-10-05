module child #(parameter P = 65'h1_0000_0000_0000_0009, parameter TAG = 0) ();
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@%0d P=%0d Q=%0d w=%0d", TAG, P, Q, w);
  case (P)
    3: begin : h3 initial #2 $display("@%0d hit3", TAG); end
    65'h1_0000_0000_0000_0009: begin : hw initial #2 $display("@%0d hitw", TAG); end
    default: begin : dd initial #2 $display("@%0d dflt", TAG); end
  endcase
endmodule
module top;
  child #(.P(3), .TAG(1)) u1();
  child #(.P(65'h1_0000_0000_0000_000A), .TAG(2)) u2();
  child #(.TAG(3)) u3();
  initial #3 $display("@top %0d %0d %0d", u1.P, u2.P, u3.P);
endmodule
