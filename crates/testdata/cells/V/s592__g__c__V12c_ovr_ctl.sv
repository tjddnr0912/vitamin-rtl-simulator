module m #(parameter P = 0) ();
  if (1) begin : gb
    case (8'd6)
      P + 1: begin : g wire [7:0] w = 8'd200; initial #1 $display("@%m k %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@%m def %0d bits=%0d", w, $bits(w)); end
    endcase
  end
endmodule
module top;
  m #(.P(5)) uA ();
  m uB ();
  initial #5 $finish;
endmodule
