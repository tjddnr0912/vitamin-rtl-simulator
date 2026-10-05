interface ifc #(parameter W = 8); logic [W-1:0] d; endinterface
module top;
  ifc #(.W(16)) bus();
  if (bus.W == 16) begin : a wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : b wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
  initial #5 $finish;
endmodule
