module top;
  localparam S = "zz";
  if (1) begin : gb
    case ("ab")
      S: begin : g wire [7:0] w = 8'd200; initial #1 $display("@s %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam S = "ab";
  end
  initial #5 $finish;
endmodule
