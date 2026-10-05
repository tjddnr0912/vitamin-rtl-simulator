module m #(parameter P = 99) ();
  if (1) begin : gb
    case (P)
      K: begin : g wire [7:0] w = 8'd200; initial #1 $display("@%m k %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@%m def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
endmodule
module top;
  m u[1:0] ();
  m #(.P(5)) v ();
  initial #5 $finish;
endmodule
