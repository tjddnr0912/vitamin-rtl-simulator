module top #(parameter P = 3);
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@ P=%0d Q=%0d w=%0d sh=%0d b=%0d", P, Q, w, P >> 60, $bits(P));
  if (P > 65'd100) begin : big
    initial #2 $display("@ big");
  end else begin : small
    initial #2 $display("@ small");
  end
  case (P)
    65'h1_0000_0000_0000_0009: begin : hw initial #3 $display("@ hitw"); end
    3: begin : h3 initial #3 $display("@ hit3"); end
    default: begin : dd initial #3 $display("@ dflt"); end
  endcase
endmodule
