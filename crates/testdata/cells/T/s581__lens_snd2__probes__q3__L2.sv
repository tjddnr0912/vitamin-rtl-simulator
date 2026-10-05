module L2;
  for (genvar i = 0; i < 2; i++) begin
    case (-1)
      32'hFFFFFFFF: begin : g wire [7:0] w = 8'd200; initial #1 $display("L2[%0d] a %0d bits=%0d", i, w, $bits(w)); end
      ((i == 0) ? 32'd5 : K): begin : g wire [7:0] w = 8'd2; initial #1 $display("L2[%0d] k %0d", i, w); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("L2[%0d] def %0d bits=%0d", i, w, $bits(w)); end
    endcase
    localparam logic [31:0] K = 32'd99;
  end
  initial #5 $finish;
endmodule
