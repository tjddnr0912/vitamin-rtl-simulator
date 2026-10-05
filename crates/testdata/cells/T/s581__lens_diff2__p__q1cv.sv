module child #(parameter P = "ab", parameter TAG = 0) (output logic [64:0] o);
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  assign o = P;
  initial #1 $display("@%0d P=%0d Q=%0d w=%0d sh=%0d b=%0d", TAG, P, Q, w, P >> 60, $bits(P));
  if (P > 65'd100) begin : bg
    initial #2 $display("@%0d big", TAG);
  end else begin : sml
    initial #2 $display("@%0d small", TAG);
  end
endmodule
module top;
  logic [64:0] o1, o2;
  child #(.P(65'h1_0000_0000_0000_0009), .TAG(1)) u1(.o(o1));
  child #(.P(7), .TAG(2)) u2(.o(o2));
  initial #4 $display("@top o1=%0d o2=%0d u1P=%0d", o1, o2, u1.P);
endmodule
