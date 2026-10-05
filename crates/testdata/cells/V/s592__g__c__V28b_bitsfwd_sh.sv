module top;
  wire [3:0] v;
  if (1) begin : gb
    case ($bits(v))
      8: begin : g wire [7:0] w = 8'd200; initial #1 $display("@k8 %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    wire [7:0] v;
  end
  initial #5 $finish;
endmodule
