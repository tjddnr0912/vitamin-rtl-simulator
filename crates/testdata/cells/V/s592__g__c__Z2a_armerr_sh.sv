module top;
  localparam logic [7:0] K = 8'd1;
  if (1) begin : gb
    case (8'd99)
      K: begin : g wire [7:0] w = 8'd200; initial #1 $display("@k %0d", w); end
      default: begin : g wire [Nope:0] w; initial #1 $display("@def"); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
