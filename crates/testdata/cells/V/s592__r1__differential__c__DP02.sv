interface bus_if #(parameter W = 8) ();
  logic [W-1:0] d;
endinterface
module sub (bus_if b);
  if ($bits(b.d) == 8) begin : a initial #1 $display("@eight"); end
  else begin : o initial #1 $display("@other"); end
endmodule
module top;
  bus_if #(.W(16)) b16();
  sub u(.b(b16));
  initial #10 $finish;
endmodule
