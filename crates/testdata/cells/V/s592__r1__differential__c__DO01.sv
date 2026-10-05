module sub #(parameter P = 1) ();
  if (P == 2) begin : a wire [7:0] w = 8'd2; initial #1 $display("@two %m bits=%0d", $bits(w)); end
  else begin : b wire [3:0] w = 4'd1; initial #1 $display("@one %m bits=%0d", $bits(w)); end
endmodule
module top;
  if (1) begin : g
    sub u();
  end
  defparam g.u.P = 2;
  initial #10 $finish;
endmodule
