interface ifc;
  if (1) begin : gb
    case (8'd99)
      K: begin : g logic [7:0] w = 8'd200; end
      default: begin : g logic [3:0] w = 4'd9; end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
endinterface
module top;
  ifc i ();
  initial #1 $display("@i %0d bits=%0d", i.gb.g.w, $bits(i.gb.g.w));
  initial #5 $finish;
endmodule
