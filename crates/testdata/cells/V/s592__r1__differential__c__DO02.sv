module sub #(parameter P = 1) ();
  if (P == 2) begin : a wire [7:0] w = 8'd2; initial #1 $display("@two %m bits=%0d", $bits(w)); end
  else begin : b wire [3:0] w = 4'd1; initial #1 $display("@one %m bits=%0d", $bits(w)); end
endmodule
module top;
  localparam integer K = 1;
  if (1) begin : g
    sub #(.P(K)) u();
    localparam integer K = 2;
  end
  initial #10 $finish;
endmodule
