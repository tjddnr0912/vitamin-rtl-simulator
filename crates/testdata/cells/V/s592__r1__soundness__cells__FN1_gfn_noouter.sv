module top;
  if (1) begin : gb
    case (8'd99)
      f(98): begin : g wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
    endcase
    function automatic integer f(input integer a); f = a + 1; endfunction
  end
  initial #5 $finish;
endmodule
