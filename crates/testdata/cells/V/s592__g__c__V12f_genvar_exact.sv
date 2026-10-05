module top;
  for (genvar i = 0; i < 4; i++) begin : L
    if (i[0] == B) begin : g wire [7:0] w = 8'd200 + i; initial #1 $display("@L%0d then %0d bits=%0d", i, w, $bits(w)); end
    else begin : g wire [3:0] w = 4'd9; initial #1 $display("@L%0d else %0d bits=%0d", i, w, $bits(w)); end
    localparam B = (i > 1);
  end
  initial #5 $finish;
endmodule
