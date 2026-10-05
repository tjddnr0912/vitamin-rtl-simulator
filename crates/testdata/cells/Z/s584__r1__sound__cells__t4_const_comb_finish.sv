module top;
  logic y;
  always_comb begin y = 1'b1; $display("K t=%0t", $time); $finish; end
endmodule
