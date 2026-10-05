module top;
  if (1) begin : gb
    case (8'd8)
      $bits(T): begin : g wire [7:0] w = 8'd200; initial #1 $display("@t %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    typedef logic [7:0] T;
  end
  initial #5 $finish;
endmodule
