module top;
  if (1) begin : gb
    if (f(98) == 99) begin : x wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : y wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
  end
  function automatic integer f(input integer a); f = a + 1; endfunction
  initial #5 $finish;
endmodule
