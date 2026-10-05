module sub #(parameter W = 8); endmodule
module top;
  sub #(.W(16)) u();
  if (u.W == 16) begin : a wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : b wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
  initial #5 $finish;
endmodule
