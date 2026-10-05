interface ifc #(parameter W = 4) ();
  if (W == 1) begin : a logic [7:0] d = 8'h11; end
  else begin : b logic [7:0] d = 8'h22; end
endinterface
module top;
  for (genvar i = 0; i < 2; i++) begin : L
    ifc #(.W(i)) x();
  end
  initial #1 $display("@%h %h", L[0].x.b.d, L[1].x.a.d);
  initial #10 $finish;
endmodule
