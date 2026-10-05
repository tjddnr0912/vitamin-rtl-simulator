module top;
  for (genvar i = 0; i < 2; i++) begin
    case (-1)
      (32'hFFFFFFFF / i): begin : a wire [7:0] w = 8'd200; initial #1 $display("@%0d a %0d bits=%0d", i, w, $bits(w)); end
      default: begin : a wire [3:0] w = 4'd9; initial #1 $display("@%0d def %0d bits=%0d", i, w, $bits(w)); end
    endcase
  end
endmodule
