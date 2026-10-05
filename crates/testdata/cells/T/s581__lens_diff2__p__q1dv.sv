module top #(parameter P = 3);
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@ P=%0d Q=%0d w=%0d sh=%0d b=%0d", P, Q, w, P >> 60, $bits(P));
  if (P > 65'd100) begin : bg
    initial #2 $display("@ big");
  end else begin : sml
    initial #2 $display("@ small");
  end
endmodule
