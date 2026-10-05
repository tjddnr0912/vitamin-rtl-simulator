module top;
  function automatic integer f(input integer a); f = a * 2; endfunction
  if (1) begin : gb
    localparam P = f(3);
    case (P)
      6: begin : g wire [7:0] w = 8'd200; initial #1 $display("@six %0d bits=%0d P=%0d", w, $bits(w), P); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d P=%0d", w, $bits(w), P); end
    endcase
  end
  initial #5 $finish;
endmodule
