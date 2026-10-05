interface bus_if #(parameter W = 8) ();
  logic [W-1:0] d;
endinterface
module sub (bus_if b);
  localparam integer LW = b.W;
  for (genvar i = 0; i < LW / 8; i++) begin : L
    initial #1 $display("@L%0d", i);
  end
endmodule
module top;
  bus_if #(.W(16)) b16();
  sub u(.b(b16));
  initial #10 $finish;
endmodule
