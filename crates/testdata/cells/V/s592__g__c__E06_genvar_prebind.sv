module top;
  for (genvar i = 0; i < 3; i++) begin : L
    localparam P = i * 2;
    case (P)
      2: begin : g wire [7:0] w = 8'd200; initial #1 $display("@L%0d two %0d bits=%0d", i, w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@L%0d def %0d bits=%0d", i, w, $bits(w)); end
    endcase
  end
  initial #5 $finish;
endmodule
